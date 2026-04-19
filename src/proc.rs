use std::{fs, io};

#[derive(Debug, Clone, Default)]
pub struct CpuStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
}

impl CpuStat {
    pub fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq
    }
    pub fn active(&self) -> u64 {
        self.total().saturating_sub(self.idle + self.iowait)
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemInfo {
    pub total_kb: u64,
    pub available_kb: u64,
}

impl MemInfo {
    pub fn used_kb(&self) -> u64 {
        self.total_kb.saturating_sub(self.available_kb)
    }
    pub fn used_pct(&self) -> u16 {
        if self.total_kb == 0 {
            return 0;
        }
        ((self.used_kb() * 100) / self.total_kb).min(100) as u16
    }
}

#[derive(Debug, Clone, Default)]
pub struct NetSample {
    pub name: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct PidStat {
    pub pid: u32,
    pub name: String,
    pub state: char,
    pub ppid: u32,
    pub utime: u64,
    pub stime: u64,
    pub threads: i64,
    pub rss: i64,
}

pub fn read_cpu_stats() -> io::Result<Vec<CpuStat>> {
    let content = fs::read_to_string("/proc/stat")?;
    let mut stats = Vec::new();
    for line in content.lines() {
        if line.len() > 4
            && line.starts_with("cpu")
            && line.as_bytes().get(3).map(|b| b.is_ascii_digit()).unwrap_or(false)
        {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 8 {
                stats.push(CpuStat {
                    user:    p[1].parse().unwrap_or(0),
                    nice:    p[2].parse().unwrap_or(0),
                    system:  p[3].parse().unwrap_or(0),
                    idle:    p[4].parse().unwrap_or(0),
                    iowait:  p[5].parse().unwrap_or(0),
                    irq:     p[6].parse().unwrap_or(0),
                    softirq: p[7].parse().unwrap_or(0),
                });
            }
        }
    }
    Ok(stats)
}

pub fn read_meminfo() -> io::Result<MemInfo> {
    let content = fs::read_to_string("/proc/meminfo")?;
    let mut info = MemInfo::default();
    for line in content.lines() {
        let mut it = line.split_whitespace();
        let key = it.next().unwrap_or("");
        let val: u64 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        match key {
            "MemTotal:"     => info.total_kb = val,
            "MemAvailable:" => info.available_kb = val,
            _ => {}
        }
    }
    Ok(info)
}

pub fn read_net_dev() -> io::Result<Vec<NetSample>> {
    let content = fs::read_to_string("/proc/net/dev")?;
    let mut result = Vec::new();
    for line in content.lines().skip(2) {
        let line = line.trim();
        if let Some(colon) = line.find(':') {
            let name = line[..colon].trim().to_string();
            let cols: Vec<&str> = line[colon + 1..].split_whitespace().collect();
            if cols.len() >= 9 {
                result.push(NetSample {
                    name,
                    rx_bytes: cols[0].parse().unwrap_or(0),
                    tx_bytes: cols[8].parse().unwrap_or(0),
                });
            }
        }
    }
    Ok(result)
}

pub fn list_pids() -> Vec<u32> {
    let mut pids = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Some(s) = entry.file_name().to_str() {
                if let Ok(pid) = s.parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
    }
    pids
}

// Parses /proc/[pid]/stat — handles comm names with spaces and parentheses
pub fn read_pid_stat(pid: u32) -> Option<PidStat> {
    let content = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let open = content.find('(')?;
    let close = content.rfind(')')?;
    let name = content[open + 1..close].to_string();
    let rest: Vec<&str> = content[close + 2..].split_whitespace().collect();
    if rest.len() < 22 {
        return None;
    }
    Some(PidStat {
        pid,
        name,
        state:   rest[0].chars().next().unwrap_or('?'),
        ppid:    rest[1].parse().unwrap_or(0),
        utime:   rest[11].parse().unwrap_or(0),
        stime:   rest[12].parse().unwrap_or(0),
        threads: rest[17].parse().unwrap_or(1),
        rss:     rest[21].parse().unwrap_or(0),
    })
}
