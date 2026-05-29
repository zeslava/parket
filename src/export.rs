use anyhow::Result;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum ExportFormat {
    Json,
    Jsonl,
    Csv,
}

impl ExportFormat {
    pub const ALL: &'static [ExportFormat] =
        &[ExportFormat::Json, ExportFormat::Jsonl, ExportFormat::Csv];

    pub fn ext(self) -> &'static str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Jsonl => "jsonl",
            ExportFormat::Csv => "csv",
        }
    }
}

pub struct ExportData<'a> {
    pub columns: &'a [String],
    pub rows: Vec<Vec<String>>,
}

pub fn run(format: ExportFormat, data: &ExportData, out_path: &str) -> Result<()> {
    let mut file = std::fs::File::create(out_path)?;
    write_to(format, data, &mut file)
}

pub fn write_to(format: ExportFormat, data: &ExportData, w: &mut dyn Write) -> Result<()> {
    match format {
        ExportFormat::Json => write_json(data, w),
        ExportFormat::Jsonl => write_jsonl(data, w),
        ExportFormat::Csv => write_csv(data, w),
    }
}

fn write_json(data: &ExportData, w: &mut dyn Write) -> Result<()> {
    w.write_all(b"[")?;
    for (i, row) in data.rows.iter().enumerate() {
        if i > 0 {
            w.write_all(b",")?;
        }
        w.write_all(b"\n  {")?;
        for (j, (col, val)) in data.columns.iter().zip(row.iter()).enumerate() {
            if j > 0 {
                w.write_all(b",")?;
            }
            write!(w, "\n    {}: {}", json_string(col), json_value(val))?;
        }
        w.write_all(b"\n  }")?;
    }
    w.write_all(b"\n]\n")?;
    Ok(())
}

fn write_jsonl(data: &ExportData, w: &mut dyn Write) -> Result<()> {
    for row in &data.rows {
        w.write_all(b"{")?;
        for (j, (col, val)) in data.columns.iter().zip(row.iter()).enumerate() {
            if j > 0 {
                w.write_all(b",")?;
            }
            write!(w, "{}: {}", json_string(col), json_value(val))?;
        }
        w.write_all(b"}\n")?;
    }
    Ok(())
}

fn write_csv(data: &ExportData, w: &mut dyn Write) -> Result<()> {
    let header: Vec<String> = data.columns.iter().map(|c| csv_field(c)).collect();
    writeln!(w, "{}", header.join(","))?;
    for row in &data.rows {
        let fields: Vec<String> = row.iter().map(|v| csv_field(v)).collect();
        writeln!(w, "{}", fields.join(","))?;
    }
    Ok(())
}

pub fn output_path(parquet_path: &str, ext: &str) -> String {
    let path = Path::new(parquet_path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let dir = path.parent().and_then(|p| p.to_str()).unwrap_or(".");
    format!("{}/{}.{}", dir, stem, ext)
}

fn json_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn json_value(s: &str) -> String {
    if s == "null" {
        return "null".to_string();
    }
    if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
        return s.to_string();
    }
    if s == "true" || s == "false" {
        return s.to_string();
    }
    json_string(s)
}

fn csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
