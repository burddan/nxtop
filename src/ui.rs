use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Sparkline, Table, Tabs},
};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    draw_tabs(f, app, chunks[0]);

    match app.tab {
        0 => draw_processes(f, app, chunks[1]),
        1 => draw_system(f, app, chunks[1]),
        2 => draw_network(f, app, chunks[1]),
        _ => {}
    }

    draw_help(f, app, chunks[2]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tabs = Tabs::new(vec!["Processos", "Sistema", "Rede"])
        .block(Block::default().borders(Borders::ALL).title(" linktop "))
        .select(app.tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, area);
}

fn draw_processes(f: &mut Frame, app: &mut App, area: Rect) {
    let show_filter = app.filter_mode || !app.filter.is_empty();
    let constraints: Vec<Constraint> = if show_filter {
        vec![Constraint::Length(3), Constraint::Min(0)]
    } else {
        vec![Constraint::Min(0)]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let (filter_area, table_area) = if show_filter {
        (Some(chunks[0]), chunks[1])
    } else {
        (None, chunks[0])
    };

    if let Some(fa) = filter_area {
        let style = if app.filter_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let input = Paragraph::new(format!(" {}", app.filter))
            .block(Block::default().borders(Borders::ALL).title(" Filtrar "))
            .style(style);
        f.render_widget(input, fa);
    }

    let tree_mode = app.tree_mode;
    let indices = app.visible_indices();
    let mode_label = if tree_mode { " [tree]" } else { "" };

    // Collect rows first to avoid borrow conflict with table_state
    let rows: Vec<Row> = indices.iter().map(|&i| {
        let p = &app.processes[i];
        let name = if tree_mode && p.depth > 0 {
            format!("{}└─ {}", "  ".repeat(p.depth - 1), p.name)
        } else {
            p.name.clone()
        };
        let cpu_color = if p.cpu_pct > 50.0 {
            Color::Red
        } else if p.cpu_pct > 20.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        Row::new([
            p.pid.to_string(),
            name,
            p.state.to_string(),
            format!("{:>5.1}", p.cpu_pct),
            format!("{:>7.1}", p.mem_mb),
            p.threads.to_string(),
        ]).style(Style::default().fg(cpu_color))
    }).collect();

    let header = Row::new(["PID", "Nome", "St", "CPU%", "MEM MB", "Threads"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let widths = [
        Constraint::Length(7),
        Constraint::Min(20),
        Constraint::Length(3),
        Constraint::Length(7),
        Constraint::Length(9),
        Constraint::Length(8),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(format!(" Processos{} ", mode_label)))
        .row_highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, table_area, &mut app.table_state);
}

fn draw_system(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let cpu_block = Block::default().borders(Borders::ALL).title(" CPU por Core ");
    let inner = cpu_block.inner(chunks[0]);
    f.render_widget(cpu_block, chunks[0]);

    let max_shown = inner.height as usize;
    for (i, history) in app.cpu_history.iter().take(max_shown).enumerate() {
        let y = inner.y + i as u16;
        let row_area = Rect::new(inner.x, y, inner.width, 1);

        let data: Vec<u64> = history.iter().copied().collect();
        let last = data.last().copied().unwrap_or(0);
        let color = if last > 80 {
            Color::Red
        } else if last > 50 {
            Color::Yellow
        } else {
            Color::Green
        };

        let row_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(row_area);

        let label = Paragraph::new(format!("CPU{:2}  {:3}%", i, last));
        f.render_widget(label, row_chunks[0]);

        let sparkline = Sparkline::default()
            .data(&data)
            .max(100)
            .style(Style::default().fg(color));
        f.render_widget(sparkline, row_chunks[1]);
    }

    let used_gb = app.mem.used_kb() as f64 / 1024.0 / 1024.0;
    let total_gb = app.mem.total_kb as f64 / 1024.0 / 1024.0;
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title(" Memória "))
        .gauge_style(Style::default().fg(Color::Blue).bg(Color::Black))
        .percent(app.mem.used_pct())
        .label(format!(
            "{:.1} GB / {:.1} GB  ({}%)",
            used_gb, total_gb, app.mem.used_pct()
        ));
    f.render_widget(gauge, chunks[1]);
}

fn draw_network(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(["Interface", "RX/s", "TX/s", "Total RX", "Total TX"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app.net.iter().map(|iface| {
        Row::new([
            iface.name.clone(),
            fmt_rate(iface.rx_bps),
            fmt_rate(iface.tx_bps),
            fmt_total(iface.rx_total),
            fmt_total(iface.tx_total),
        ])
    }).collect();

    let widths = [
        Constraint::Length(15),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Rede "));
    f.render_widget(table, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.tab == 0 {
        if app.filter_mode {
            " ESC/Enter: fechar filtro"
        } else {
            " j/k↑↓: nav  /: filtrar  t: tree  x: kill  Tab: aba  q: sair"
        }
    } else {
        " Tab: próxima aba  q: sair"
    };
    let help = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, area);
}

fn fmt_rate(bps: u64) -> String {
    if bps >= 1024 * 1024 {
        format!("{:.1} MB/s", bps as f64 / 1_048_576.0)
    } else if bps >= 1024 {
        format!("{:.1} KB/s", bps as f64 / 1024.0)
    } else {
        format!("{} B/s", bps)
    }
}

fn fmt_total(b: u64) -> String {
    if b >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", b as f64 / 1_073_741_824.0)
    } else if b >= 1024 * 1024 {
        format!("{:.1} MB", b as f64 / 1_048_576.0)
    } else if b >= 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{} B", b)
    }
}
