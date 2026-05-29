mod app;
mod data;
mod export;
mod ui;

use anyhow::Result;
use app::{App, InputMode};
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use export::{ExportData, ExportFormat};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "parket", about = "Parquet file viewer and converter")]
struct Cli {
    /// Parquet file to open in TUI
    file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Export parquet file to JSON, JSONL, or CSV
    Export {
        /// Input parquet file
        file: PathBuf,
        /// Output format
        #[arg(short, long, default_value = "json")]
        format: CliFormat,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Clone, ValueEnum)]
enum CliFormat {
    Json,
    Jsonl,
    Csv,
}

impl From<CliFormat> for ExportFormat {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Json => ExportFormat::Json,
            CliFormat::Jsonl => ExportFormat::Jsonl,
            CliFormat::Csv => ExportFormat::Csv,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Export {
            file,
            format,
            output,
        }) => run_export(file, format, output),
        None => {
            let path = cli
                .file
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| {
                    eprintln!("Usage: parket <file.parquet>");
                    std::process::exit(1);
                });
            run_tui(path)
        }
    }
}

fn run_export(file: PathBuf, fmt: CliFormat, output: Option<PathBuf>) -> Result<()> {
    let path = file.to_string_lossy().into_owned();
    let (schema, batches) = data::load(&path)?;

    let columns: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();

    let rows: Vec<Vec<String>> = batches
        .iter()
        .flat_map(|batch| {
            (0..batch.num_rows()).map(|ri| {
                (0..batch.num_columns())
                    .map(|ci| data::cell_to_string(batch.column(ci), ri))
                    .collect()
            })
        })
        .collect();

    let data = ExportData {
        columns: &columns,
        rows,
    };

    let format: ExportFormat = fmt.into();

    match output {
        Some(out_path) => {
            export::run(format, &data, out_path.to_str().unwrap())?;
            eprintln!("Exported to {}", out_path.display());
        }
        None => {
            let stdout = io::stdout();
            export::write_to(format, &data, &mut stdout.lock())?;
        }
    }

    Ok(())
}

fn run_tui(path: String) -> Result<()> {
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

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
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
                    KeyCode::Backspace => {
                        app.search_pop();
                    }
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
                        KeyCode::Esc => {
                            app.clear_filter();
                            app.status_msg = None;
                        }
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
