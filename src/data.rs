use anyhow::Result;
use arrow::array::Array;
use arrow::array::RecordBatch;
use arrow::datatypes::SchemaRef;
use arrow::util::display::{ArrayFormatter, FormatOptions};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;

pub fn cell_to_string(arr: &dyn Array, row: usize) -> String {
    if arr.is_null(row) {
        return "null".to_string();
    }
    let opts = FormatOptions::default().with_display_error(true);
    match ArrayFormatter::try_new(arr, &opts) {
        Ok(fmt) => fmt.value(row).to_string(),
        Err(_) => "?".to_string(),
    }
}

pub fn load(path: &str) -> Result<(SchemaRef, Vec<RecordBatch>)> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let schema = builder.schema().clone();
    let reader = builder.build()?;
    let batches = reader.collect::<Result<Vec<_>, _>>()?;
    Ok((schema, batches))
}
