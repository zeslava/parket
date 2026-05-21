use crate::app::{App, InputMode, Mode};
use crate::export::ExportFormat;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};
use std::fmt::Write as _;

pub fn draw(f: &mut Frame, app: &App) {
    let searching = matches!(app.input_mode, InputMode::Search);
    let exporting = matches!(app.input_mode, InputMode::Export);

    let mut constraints = vec![
        Constraint::Length(3), // tabs
        Constraint::Min(0),    // table
    ];
    if searching || exporting {
        constraints.push(Constraint::Length(1)); // input bar
    }
    constraints.push(Constraint::Length(1)); // status bar

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    let table_area = chunks[1];
    let bar_area = if searching || exporting { Some(chunks[2]) } else { None };
    let status_area = chunks[if searching || exporting { 3 } else { 2 }];

    // Tabs
    let titles = vec![
        Line::from(Span::raw("Data")),
        Line::from(Span::raw("Schema")),
    ];
    let selected = match app.mode {
        Mode::Data => 0,
        Mode::Schema => 1,
    };
    let tabs = Tabs::new(titles)
        .select(selected)
        .block(Block::default().borders(Borders::ALL).title(app.path.as_str()))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    match app.mode {
        Mode::Schema => draw_schema(f, app, table_area),
        Mode::Data => draw_data(f, app, table_area),
    }

    // Search / Export bar
    if let Some(area) = bar_area {
        if searching {
            let bar = Paragraph::new(format!("/{}", app.search_input))
                .style(Style::default().bg(Color::Blue).fg(Color::White));
            f.render_widget(bar, area);
        } else if exporting {
            let mut line = String::from(" Export: ");
            for (i, fmt) in ExportFormat::ALL.iter().enumerate() {
                if i > 0 { line.push_str("  "); }
                if i == app.export_selected {
                    let _ = write!(line, "[{}]", fmt.ext().to_uppercase());
                } else {
                    line.push_str(fmt.ext());
                }
            }
            line.push_str("  ←/→ select  Enter=confirm  Esc=cancel");
            let bar = Paragraph::new(line)
                .style(Style::default().bg(Color::Green).fg(Color::Black));
            f.render_widget(bar, area);
        }
    }

    // Status bar
    let status = if let Some(msg) = &app.status_msg {
        format!(" {}", msg)
    } else {
        let filter_hint = if !app.active_filter.is_empty() {
            format!(" [filter: {}] {}/{}", app.active_filter, app.total_rows(), app.total_rows())
        } else {
            String::new()
        };
        match app.mode {
            Mode::Schema => format!(" Schema | {} columns", app.schema.fields().len()),
            Mode::Data => format!(
                " Row {}/{} | {} cols{}  g/G=first/last  PgUp/Dn=page  /=search  e=export  Esc=clear",
                if app.total_rows() == 0 { 0 } else { app.row_offset + 1 },
                app.total_rows(),
                app.schema.fields().len(),
                filter_hint,
            ),
        }
    };
    let statusbar = Paragraph::new(status)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(statusbar, status_area);
}

fn draw_schema(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Type").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Nullable").style(Style::default().add_modifier(Modifier::BOLD)),
    ])
    .style(Style::default().fg(Color::Cyan));

    let visible = area.height.saturating_sub(3) as usize;
    let rows: Vec<Row> = app
        .schema
        .fields()
        .iter()
        .skip(app.row_offset)
        .take(visible)
        .map(|f| {
            Row::new(vec![
                Cell::from(f.name().as_str().to_string()),
                Cell::from(format!("{}", f.data_type())),
                Cell::from(if f.is_nullable() { "yes" } else { "no" }),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(45),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Schema"));
    f.render_widget(table, area);
}

fn draw_data(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let visible_rows = area.height.saturating_sub(3) as usize;
    let num_cols = app.schema.fields().len();
    let visible_cols = ((area.width.saturating_sub(2)) / 16).max(1) as usize;

    let col_start = app.col_offset;
    let col_end = (col_start + visible_cols).min(num_cols);

    let header_cells: Vec<Cell> = app
        .schema
        .fields()
        .iter()
        .enumerate()
        .filter(|(i, _)| *i >= col_start && *i < col_end)
        .map(|(_, f)| {
            Cell::from(f.name().as_str().to_string())
                .style(Style::default().add_modifier(Modifier::BOLD))
        })
        .collect();
    let header = Row::new(header_cells).style(Style::default().fg(Color::Cyan));

    let data = app.data_rows(app.row_offset, visible_rows);
    let needle = app.active_filter.to_lowercase();

    let rows: Vec<Row> = data
        .iter()
        .map(|row| {
            let cells: Vec<Cell> = row
                .iter()
                .enumerate()
                .filter(|(i, _)| *i >= col_start && *i < col_end)
                .map(|(_, v)| {
                    let matches = !needle.is_empty() && v.to_lowercase().contains(&needle);
                    let cell = Cell::from(v.as_str().to_string());
                    if matches {
                        cell.style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    } else {
                        cell
                    }
                })
                .collect();
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> = (col_start..col_end)
        .map(|_| Constraint::Length(16))
        .collect();

    let title = if !app.active_filter.is_empty() {
        format!("Data [filter: {}]", app.active_filter)
    } else {
        "Data".to_string()
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title));
    f.render_widget(table, area);
}
