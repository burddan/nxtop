mod app;
mod proc;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    time::{Duration, Instant},
};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let tick = Duration::from_millis(1000);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        let timeout = tick.checked_sub(last_tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if app.filter_mode {
                    match key.code {
                        KeyCode::Esc | KeyCode::Enter => app.filter_mode = false,
                        KeyCode::Backspace => { app.filter.pop(); }
                        KeyCode::Char(c) => app.filter.push(c),
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Char('t') => app.toggle_tree(),
                        KeyCode::Char('/') => {
                            app.filter_mode = true;
                            app.filter.clear();
                        }
                        KeyCode::Esc => {
                            app.filter.clear();
                            app.filter_mode = false;
                        }
                        KeyCode::Char('x') => app.kill_selected(),
                        KeyCode::Char('r') => app.refresh(),
                        _ => {}
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick {
            app.refresh();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}
