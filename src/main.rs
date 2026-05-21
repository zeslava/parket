mod app;
mod data;
mod export;
mod ui;

use anyhow::Result;
use app::{App, InputMode};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("Usage: parket <file.parquet>");

    let (schema, batches) = data::load(&path)?;
    let mut app = App::new(path, schema, batches);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.input_mode {
                InputMode::Search => match key.code {
                    KeyCode::Enter => app.apply_search(),
                    KeyCode::Esc => app.cancel_search(),
                    KeyCode::Backspace => { app.search_pop(); }
                    KeyCode::Char(c) => app.search_push(c),
                    _ => {}
                },
                InputMode::Export => match key.code {
                    KeyCode::Enter => app.confirm_export(),
                    KeyCode::Esc => app.cancel_export(),
                    KeyCode::Right | KeyCode::Down => app.export_next(),
                    KeyCode::Left | KeyCode::Up => app.export_prev(),
                    _ => {}
                },
                InputMode::Normal => {
                    let height = terminal.size()?.height;
                    let visible_rows = height.saturating_sub(4) as usize;
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Esc => { app.clear_filter(); app.status_msg = None; }
                        KeyCode::Tab => app.toggle_mode(),
                        KeyCode::Down => app.scroll_down(1),
                        KeyCode::Up => app.scroll_up(1),
                        KeyCode::PageDown => app.scroll_down(visible_rows),
                        KeyCode::PageUp => app.scroll_up(visible_rows),
                        KeyCode::Char('g') => app.goto_first(),
                        KeyCode::Char('G') => app.goto_last(visible_rows),
                        KeyCode::Right => app.scroll_right(),
                        KeyCode::Left => app.scroll_left(),
                        KeyCode::Char('/') => app.enter_search(),
                        KeyCode::Char('e') => app.enter_export(),
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}
