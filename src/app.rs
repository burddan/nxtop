use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};
use ratatui::widgets::TableState;
use crate::proc::{self, CpuStat, MemInfo, NetSample};

const HISTORY: usize = 60;
const PAGE_BYTES: u64 = 4096;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: char,
    pub ppid: u32,
    pub cpu_pct: f32,
    pub mem_mb: f32,
    pub threads: i64,
    pub depth: usize,
}

#[derive(Debug, Clone, Default)]
pub struct NetIfaceDisplay {
    pub name: String,
    pub rx_bps: u64,
    pub tx_bps: u64,
    pub rx_total: u64,
    pub tx_total: u64,
}

pub struct App {
    pub tab: usize,
    pub processes: Vec<ProcessInfo>,
    pub table_state: TableState,
    pub filter: String,
    pub filter_mode: bool,
    pub tree_mode: bool,
    pub cpu_history: Vec<VecDeque<u64>>,
    pub mem: MemInfo,
    pub net: Vec<NetIfaceDisplay>,
    prev_cpu: Vec<CpuStat>,
    prev_ticks: HashMap<u32, u64>,
    prev_total: u64,
    prev_net: HashMap<String, NetSample>,
    last_refresh: Instant,
}

impl App {
    pub fn new() -> Self {
        let cpu = proc::read_cpu_stats().unwrap_or_default();
        let cores = cpu.len().max(1);
        let mut app = Self {
            tab: 0,
            processes: vec![],
            table_state: TableState::default(),
            filter: String::new(),
            filter_mode: false,
            tree_mode: false,
            cpu_history: vec![VecDeque::from(vec![0u64; HISTORY]); cores],
            mem: MemInfo::default(),
            net: vec![],
            prev_cpu: cpu,
            prev_ticks: HashMap::new(),
            prev_total: 0,
            prev_net: HashMap::new(),
            last_refresh: Instant::now(),
        };
        app.refresh();
        app.table_state.select(Some(0));
        app
    }

    pub fn refresh(&mut self) {
        let elapsed = self.last_refresh.elapsed().as_secs_f64().max(0.001);
        self.last_refresh = Instant::now();
        self.update_cpu();
        self.update_mem();
        self.update_net(elapsed);
        self.update_procs();
    }

    fn update_cpu(&mut self) {
        let Ok(new) = proc::read_cpu_stats() else { return };
        while self.cpu_history.len() < new.len() {
            self.cpu_history.push(VecDeque::from(vec![0u64; HISTORY]));
        }
        for (i, (prev, curr)) in self.prev_cpu.iter().zip(new.iter()).enumerate() {
            let total_d = curr.total().saturating_sub(prev.total());
            let active_d = curr.active().saturating_sub(prev.active());
            let pct = if total_d > 0 { (active_d * 100 / total_d).min(100) } else { 0 };
            if self.cpu_history[i].len() >= HISTORY {
                self.cpu_history[i].pop_front();
            }
            self.cpu_history[i].push_back(pct);
        }
        self.prev_cpu = new;
    }

    fn update_mem(&mut self) {
        if let Ok(m) = proc::read_meminfo() {
            self.mem = m;
        }
    }

    fn update_net(&mut self, elapsed: f64) {
        let Ok(samples) = proc::read_net_dev() else { return };
        self.net = samples.iter().map(|s| {
            let (rx_bps, tx_bps) = self.prev_net.get(&s.name).map(|prev| (
                ((s.rx_bytes.saturating_sub(prev.rx_bytes)) as f64 / elapsed) as u64,
                ((s.tx_bytes.saturating_sub(prev.tx_bytes)) as f64 / elapsed) as u64,
            )).unwrap_or((0, 0));
            NetIfaceDisplay {
                name: s.name.clone(),
                rx_bps,
                tx_bps,
                rx_total: s.rx_bytes,
                tx_total: s.tx_bytes,
            }
        }).collect();
        self.prev_net = samples.into_iter().map(|s| (s.name.clone(), s)).collect();
    }

    fn update_procs(&mut self) {
        let pids = proc::list_pids();
        let num_cores = self.prev_cpu.len().max(1) as f32;
        // prev_cpu was just updated to current readings in update_cpu()
        let total_now: u64 = self.prev_cpu.iter().map(|c| c.total()).sum();
        let total_d = total_now.saturating_sub(self.prev_total);

        let mut new_ticks: HashMap<u32, u64> = HashMap::new();
        let mut raw: Vec<(proc::PidStat, f32)> = Vec::with_capacity(pids.len());

        for pid in pids {
            if let Some(stat) = proc::read_pid_stat(pid) {
                let ticks = stat.utime + stat.stime;
                let prev = self.prev_ticks.get(&pid).copied().unwrap_or(ticks);
                let delta = ticks.saturating_sub(prev);
                let cpu = if total_d > 0 {
                    (delta as f32 / total_d as f32 * num_cores * 100.0).min(100.0)
                } else {
                    0.0
                };
                new_ticks.insert(pid, ticks);
                raw.push((stat, cpu));
            }
        }

        self.prev_ticks = new_ticks;
        self.prev_total = total_now;

        raw.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        self.processes = if self.tree_mode {
            build_tree(raw)
        } else {
            raw.into_iter().map(|(s, cpu)| to_proc_info(s, cpu, 0)).collect()
        };
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            (0..self.processes.len()).collect()
        } else {
            let f = self.filter.to_lowercase();
            self.processes.iter().enumerate()
                .filter(|(_, p)| p.name.to_lowercase().contains(&f) || p.pid.to_string().contains(&self.filter))
                .map(|(i, _)| i)
                .collect()
        }
    }

    pub fn next(&mut self) {
        let n = self.visible_indices().len();
        if n == 0 { return; }
        let i = self.table_state.selected().map(|i| (i + 1).min(n - 1)).unwrap_or(0);
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = self.table_state.selected().map(|i| i.saturating_sub(1)).unwrap_or(0);
        self.table_state.select(Some(i));
    }

    pub fn kill_selected(&mut self) {
        let indices = self.visible_indices();
        if let Some(i) = self.table_state.selected() {
            if let Some(&proc_idx) = indices.get(i) {
                if let Some(p) = self.processes.get(proc_idx) {
                    let pid = p.pid.to_string();
                    let _ = std::process::Command::new("kill").args(["-9", &pid]).output();
                }
            }
        }
        self.refresh();
    }

    pub fn toggle_tree(&mut self) {
        self.tree_mode = !self.tree_mode;
        self.refresh();
    }

    pub fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % 3;
    }

    pub fn prev_tab(&mut self) {
        self.tab = if self.tab == 0 { 2 } else { self.tab - 1 };
    }
}

fn to_proc_info(s: proc::PidStat, cpu: f32, depth: usize) -> ProcessInfo {
    ProcessInfo {
        pid: s.pid,
        name: s.name,
        state: s.state,
        ppid: s.ppid,
        cpu_pct: cpu,
        mem_mb: (s.rss.max(0) as u64 * PAGE_BYTES) as f32 / 1024.0 / 1024.0,
        threads: s.threads,
        depth,
    }
}

fn build_tree(raw: Vec<(proc::PidStat, f32)>) -> Vec<ProcessInfo> {
    let procs: Vec<ProcessInfo> = raw.into_iter()
        .map(|(s, cpu)| to_proc_info(s, cpu, 0))
        .collect();

    let all_pids: HashSet<u32> = procs.iter().map(|p| p.pid).collect();
    let idx_by_pid: HashMap<u32, usize> = procs.iter().enumerate().map(|(i, p)| (p.pid, i)).collect();

    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in &procs {
        if all_pids.contains(&p.ppid) {
            children.entry(p.ppid).or_default().push(p.pid);
        }
    }

    let mut roots: Vec<u32> = procs.iter()
        .filter(|p| !all_pids.contains(&p.ppid))
        .map(|p| p.pid)
        .collect();
    roots.sort_unstable();

    let mut result = Vec::with_capacity(procs.len());
    let mut stack: Vec<(u32, usize)> = roots.into_iter().rev().map(|pid| (pid, 0)).collect();

    while let Some((pid, depth)) = stack.pop() {
        if let Some(&idx) = idx_by_pid.get(&pid) {
            let mut p = procs[idx].clone();
            p.depth = depth;
            result.push(p);
            if let Some(child_pids) = children.get(&pid) {
                let mut sorted = child_pids.clone();
                sorted.sort_unstable();
                for child in sorted.into_iter().rev() {
                    stack.push((child, depth + 1));
                }
            }
        }
    }
    result
}
