use arrow::array::RecordBatch;
use arrow::datatypes::SchemaRef;
use crate::export::{self, ExportData, ExportFormat};

pub enum Mode {
    Schema,
    Data,
}

pub enum InputMode {
    Normal,
    Search,
    Export,
}


pub struct App {
    pub path: String,
    pub schema: SchemaRef,
    pub batches: Vec<RecordBatch>,
    pub mode: Mode,
    pub input_mode: InputMode,
    pub row_offset: usize,
    pub col_offset: usize,
    /// All rows flattened: (batch_idx, row_idx)
    all_indices: Vec<(usize, usize)>,
    /// Indices after filter applied (or all_indices if no filter)
    pub filtered_indices: Vec<(usize, usize)>,
    pub search_input: String,
    pub active_filter: String,
    pub status_msg: Option<String>,
    pub export_selected: usize,
}

impl App {
    pub fn new(path: String, schema: SchemaRef, batches: Vec<RecordBatch>) -> Self {
        let all_indices: Vec<(usize, usize)> = batches
            .iter()
            .enumerate()
            .flat_map(|(bi, b)| (0..b.num_rows()).map(move |ri| (bi, ri)))
            .collect();
        let filtered_indices = all_indices.clone();
        Self {
            path,
            schema,
            batches,
            mode: Mode::Data,
            input_mode: InputMode::Normal,
            row_offset: 0,
            col_offset: 0,
            all_indices,
            filtered_indices,
            search_input: String::new(),
            active_filter: String::new(),
            status_msg: None,
            export_selected: 0,
        }
    }

    pub fn total_rows(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Schema => Mode::Data,
            Mode::Data => Mode::Schema,
        };
        self.row_offset = 0;
        self.col_offset = 0;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = match self.mode {
            Mode::Schema => self.schema.fields().len().saturating_sub(1),
            Mode::Data => self.total_rows().saturating_sub(1),
        };
        self.row_offset = (self.row_offset + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.row_offset = self.row_offset.saturating_sub(n);
    }

    pub fn goto_first(&mut self) {
        self.row_offset = 0;
    }

    pub fn goto_last(&mut self, visible_rows: usize) {
        let total = match self.mode {
            Mode::Schema => self.schema.fields().len(),
            Mode::Data => self.total_rows(),
        };
        self.row_offset = total.saturating_sub(visible_rows);
    }

    pub fn scroll_right(&mut self) {
        let max = self.schema.fields().len().saturating_sub(1);
        if self.col_offset < max {
            self.col_offset += 1;
        }
    }

    pub fn scroll_left(&mut self) {
        if self.col_offset > 0 {
            self.col_offset -= 1;
        }
    }

    pub fn enter_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_input.clone_from(&self.active_filter);
    }

    pub fn search_push(&mut self, c: char) {
        self.search_input.push(c);
    }

    pub fn search_pop(&mut self) {
        self.search_input.pop();
    }

    pub fn apply_search(&mut self) {
        self.active_filter.clone_from(&self.search_input);
        self.input_mode = InputMode::Normal;
        self.rebuild_filter();
        self.row_offset = 0;
    }

    pub fn cancel_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_input.clone_from(&self.active_filter);
    }

    pub fn clear_filter(&mut self) {
        self.active_filter.clear();
        self.search_input.clear();
        self.input_mode = InputMode::Normal;
        self.filtered_indices.clone_from(&self.all_indices);
        self.row_offset = 0;
    }

    fn rebuild_filter(&mut self) {
        if self.active_filter.is_empty() {
            self.filtered_indices.clone_from(&self.all_indices);
            return;
        }
        let needle = self.active_filter.to_lowercase();
        self.filtered_indices = self
            .all_indices
            .iter()
            .copied()
            .filter(|&(bi, ri)| {
                let batch = &self.batches[bi];
                (0..batch.num_columns()).any(|col| {
                    let arr = batch.column(col);
                    arrow::util::display::array_value_to_string(arr, ri)
                        .map(|s| s.to_lowercase().contains(&needle))
                        .unwrap_or(false)
                })
            })
            .collect();
    }

    pub fn enter_export(&mut self) {
        self.input_mode = InputMode::Export;
        self.export_selected = 0;
        self.status_msg = None;
    }

    pub fn export_next(&mut self) {
        self.export_selected = (self.export_selected + 1) % ExportFormat::ALL.len();
    }

    pub fn export_prev(&mut self) {
        self.export_selected = self.export_selected
            .checked_sub(1)
            .unwrap_or(ExportFormat::ALL.len() - 1);
    }

    pub fn confirm_export(&mut self) {
        let fmt = ExportFormat::ALL[self.export_selected];
        self.input_mode = InputMode::Normal;
        self.export(fmt);
    }

    pub fn cancel_export(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn export(&mut self, fmt: ExportFormat) {
        let columns: Vec<String> = self.schema.fields().iter().map(|f| f.name().clone()).collect();
        let rows = self.all_filtered_rows();
        let data = ExportData { columns: &columns, rows };
        let out_path = export::output_path(&self.path, fmt.ext());
        let result = export::run(fmt, &data, &out_path);
        self.status_msg = Some(match result {
            Ok(_) => format!("Exported to {}", out_path),
            Err(e) => format!("Export error: {}", e),
        });
    }

    fn all_filtered_rows(&self) -> Vec<Vec<String>> {
        self.filtered_indices
            .iter()
            .map(|&(bi, ri)| {
                let batch = &self.batches[bi];
                (0..batch.num_columns())
                    .map(|col| {
                        let arr = batch.column(col);
                        arrow::util::display::array_value_to_string(arr, ri)
                            .unwrap_or_else(|_| "?".to_string())
                    })
                    .collect()
            })
            .collect()
    }

    pub fn data_rows(&self, offset: usize, count: usize) -> Vec<Vec<String>> {
        self.filtered_indices
            .iter()
            .skip(offset)
            .take(count)
            .map(|&(bi, ri)| {
                let batch = &self.batches[bi];
                (0..batch.num_columns())
                    .map(|col| {
                        let arr = batch.column(col);
                        arrow::util::display::array_value_to_string(arr, ri)
                            .unwrap_or_else(|_| "?".to_string())
                    })
                    .collect()
            })
            .collect()
    }
}
