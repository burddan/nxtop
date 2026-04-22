use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Row, Sparkline, Table, Tabs},
};
use crate::app::App;

struct Theme;
impl Theme {
    const BG:      Color = Color::Black;
    const PRIMARY: Color = Color::Cyan;
    const ACCENT:  Color = Color::Magenta;
    const OK:      Color = Color::Green;
    const WARN:    Color = Color::Yellow;
    const CRIT:    Color = Color::Red;
    const DIM:     Color = Color::DarkGray;

    fn block(title: &str) -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Self::PRIMARY))
            .title(Span::styled(
                format!("[ {} ]", title),
                Style::default().fg(Self::ACCENT).add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Self::BG))
    }

    fn cpu_color(pct: u64) -> Color {
        match pct {
            0..=40  => Self::OK,
            41..=75 => Self::WARN,
            _       => Self::CRIT,
        }
    }

    fn mem_color(pct: u16) -> Color {
        match pct {
            0..=50  => Self::OK,
            51..=80 => Self::WARN,
            _       => Self::CRIT,
        }
    }

    fn temp_color(c: u32) -> Color {
        match c {
            0..=59  => Self::OK,
            60..=79 => Self::WARN,
            _       => Self::CRIT,
        }
    }

    fn bar(pct: u16, width: usize) -> String {
        let filled = (pct as usize * width / 100).min(width);
        let bar: String = (0..width).map(|i| if i < filled { '█' } else { '░' }).collect();
        format!("[{}] {:3}%", bar, pct)
    }
}

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
    let tabs = Tabs::new(vec!["  Processos  ", "  Sistema  ", "  Rede  "])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Theme::PRIMARY))
                .title(Span::styled(
                    "[ nxtop ]",
                    Style::default().fg(Theme::PRIMARY).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(Theme::BG)),
        )
        .select(app.tab)
        .style(Style::default().fg(Theme::DIM).bg(Theme::BG))
        .highlight_style(
            Style::default()
                .fg(Theme::BG)
                .bg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        );
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
            Style::default().fg(Theme::ACCENT)
        } else {
            Style::default().fg(Theme::DIM)
        };
        let input = Paragraph::new(format!(" {}_", app.filter))
            .block(Theme::block("Filtrar"))
            .style(style);
        f.render_widget(input, fa);
    }

    let tree_mode = app.tree_mode;
    let indices = app.visible_indices();
    let title = if tree_mode { "Processos [ TREE ]" } else { "Processos" };

    let rows: Vec<Row> = indices.iter().map(|&i| {
        let p = &app.processes[i];
        let name = if tree_mode && p.depth > 0 {
            format!("{}└─ {}", "  ".repeat(p.depth - 1), p.name)
        } else {
            p.name.clone()
        };
        let cpu_color = if p.cpu_pct > 50.0 {
            Theme::CRIT
        } else if p.cpu_pct > 20.0 {
            Theme::WARN
        } else {
            Theme::OK
        };
        Row::new([
            p.pid.to_string(),
            name,
            p.state.to_string(),
            format!("{:>5.1}", p.cpu_pct),
            format!("{:>7.1}", p.mem_mb),
            p.threads.to_string(),
        ])
        .style(Style::default().fg(cpu_color).bg(Theme::BG))
    }).collect();

    let header = Row::new(["PID", "Nome", "St", "CPU%", "MEM MB", "Threads"])
        .style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        );

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
        .block(Theme::block(title))
        .row_highlight_style(
            Style::default()
                .fg(Theme::PRIMARY)
                .bg(Color::Rgb(20, 20, 40))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, table_area, &mut app.table_state);
}

fn draw_system(f: &mut Frame, app: &App, area: Rect) {
    let gpu_height   = if app.gpus.is_empty()   { 0 } else { (app.gpus.len()  * 2 + 2) as u16 };
    let temp_height  = if app.temps.is_empty()  { 0 } else { (app.temps.len() + 2)     as u16 };

    let mut constraints = vec![Constraint::Min(0), Constraint::Length(3)];
    if gpu_height  > 0 { constraints.push(Constraint::Length(gpu_height));  }
    if temp_height > 0 { constraints.push(Constraint::Length(temp_height)); }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let cpu_block = Theme::block("CPU por Core");
    let inner = cpu_block.inner(chunks[0]);
    f.render_widget(cpu_block, chunks[0]);

    let max_shown = inner.height as usize;
    for (i, history) in app.cpu_history.iter().take(max_shown).enumerate() {
        let y = inner.y + i as u16;
        let row_area = Rect::new(inner.x, y, inner.width, 1);

        let data: Vec<u64> = history.iter().copied().collect();
        let last = data.last().copied().unwrap_or(0);
        let color = Theme::cpu_color(last);

        let row_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(22), Constraint::Min(0)])
            .split(row_area);

        let bar = Theme::bar(last as u16, 8);
        let label = Paragraph::new(format!("CPU{:2}  {}", i, bar))
            .style(Style::default().fg(color).bg(Theme::BG));
        f.render_widget(label, row_chunks[0]);

        let sparkline = Sparkline::default()
            .data(&data)
            .max(100)
            .style(Style::default().fg(color).bg(Theme::BG));
        f.render_widget(sparkline, row_chunks[1]);
    }

    let used_gb = app.mem.used_kb() as f64 / 1024.0 / 1024.0;
    let total_gb = app.mem.total_kb as f64 / 1024.0 / 1024.0;
    let pct = app.mem.used_pct();
    let mem_color = Theme::mem_color(pct);

    let gauge = Gauge::default()
        .block(Theme::block("Memória"))
        .gauge_style(Style::default().fg(mem_color).bg(Color::Rgb(10, 10, 10)))
        .percent(pct)
        .label(Span::styled(
            format!("{:.1} GB / {:.1} GB  ({}%)", used_gb, total_gb, pct),
            Style::default().fg(Theme::BG).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(gauge, chunks[1]);

    let mut chunk_idx = 2usize;
    if !app.gpus.is_empty() {
        draw_gpu(f, app, chunks[chunk_idx]);
        chunk_idx += 1;
    }
    if !app.temps.is_empty() {
        draw_temps(f, app, chunks[chunk_idx]);
    }
}

fn draw_gpu(f: &mut Frame, app: &App, area: Rect) {
    let gpu_block = Theme::block("GPU");
    let inner = gpu_block.inner(area);
    f.render_widget(gpu_block, area);

    for (i, (gpu, history)) in app.gpus.iter().zip(app.gpu_history.iter()).enumerate() {
        let y_base = inner.y + (i * 2) as u16;
        if y_base + 1 >= inner.y + inner.height { break; }

        let util_row = Rect::new(inner.x, y_base,     inner.width, 1);
        let vram_row = Rect::new(inner.x, y_base + 1, inner.width, 1);

        let color = Theme::cpu_color(gpu.util_pct as u64);
        let bar   = Theme::bar(gpu.util_pct as u16, 8);
        let temp  = gpu.temp_c.map(|t| format!("  {:>3}°C", t)).unwrap_or_else(|| "       ".into());
        let short_name = gpu.name.chars().take(12).collect::<String>();
        let label_text = format!("{:<12}  {}{}", short_name, bar, temp);

        let row_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(0)])
            .split(util_row);

        f.render_widget(
            Paragraph::new(label_text).style(Style::default().fg(color).bg(Theme::BG)),
            row_chunks[0],
        );

        let data: Vec<u64> = history.iter().copied().collect();
        f.render_widget(
            Sparkline::default().data(&data).max(100)
                .style(Style::default().fg(color).bg(Theme::BG)),
            row_chunks[1],
        );

        if gpu.vram_total_mb > 0 {
            let vram_pct   = ((gpu.vram_used_mb * 100) / gpu.vram_total_mb.max(1)).min(100) as u16;
            let vram_color = Theme::mem_color(vram_pct);
            let vram_bar   = Theme::bar(vram_pct, 8);
            let vram_text  = format!(
                "VRAM   {}  {:.1} / {:.1} GB",
                vram_bar,
                gpu.vram_used_mb  as f64 / 1024.0,
                gpu.vram_total_mb as f64 / 1024.0,
            );
            f.render_widget(
                Paragraph::new(vram_text).style(Style::default().fg(vram_color).bg(Theme::BG)),
                vram_row,
            );
        }
    }
}

fn draw_temps(f: &mut Frame, app: &App, area: Rect) {
    let block = Theme::block("Temperatura");
    let inner = block.inner(area);
    f.render_widget(block, area);

    for (i, t) in app.temps.iter().enumerate() {
        let y = inner.y + i as u16;
        if y >= inner.y + inner.height { break; }
        let row = Rect::new(inner.x, y, inner.width, 1);

        let color    = Theme::temp_color(t.temp_c);
        let bar      = Theme::bar(t.temp_c.min(100) as u16, 8);
        let src      = format!("{:<6}", t.source);
        let label    = format!("{:<22}", t.label);
        let line     = format!("{}  {}  {:>3}°C  {}", src, label, t.temp_c, bar);

        f.render_widget(
            Paragraph::new(line).style(Style::default().fg(color).bg(Theme::BG)),
            row,
        );
    }
}

fn draw_network(f: &mut Frame, app: &App, area: Rect) {
    let header = Row::new(["Interface", "RX/s", "TX/s", "Total RX", "Total TX"])
        .style(
            Style::default()
                .fg(Theme::ACCENT)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::REVERSED),
        );

    let rows: Vec<Row> = app.net.iter().map(|iface| {
        Row::new([
            iface.name.clone(),
            fmt_rate(iface.rx_bps),
            fmt_rate(iface.tx_bps),
            fmt_total(iface.rx_total),
            fmt_total(iface.tx_total),
        ])
        .style(Style::default().fg(Theme::PRIMARY).bg(Theme::BG))
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
        .block(Theme::block("Rede"));
    f.render_widget(table, area);
}

fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let text = if app.tab == 0 {
        if app.filter_mode {
            " ESC/Enter: fechar filtro"
        } else {
            " j/k↑↓: nav  │  /: filtrar  │  t: tree  │  x: kill  │  Tab: aba  │  q: sair"
        }
    } else {
        " Tab: próxima aba  │  q: sair"
    };
    let help = Paragraph::new(text)
        .style(Style::default().fg(Theme::DIM).bg(Theme::BG));
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
