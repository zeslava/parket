use anyhow::Result;
use arrow::array::{Array, RecordBatch, StringArray};
use arrow::compute::cast;
use arrow::datatypes::{DataType, SchemaRef};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;

pub fn cell_to_string(arr: &dyn Array, row: usize) -> String {
    if arr.is_null(row) {
        return "null".to_string();
    }
    let is_temporal = matches!(
        arr.data_type(),
        DataType::Timestamp(_, _)
            | DataType::Date32
            | DataType::Date64
            | DataType::Time32(_)
            | DataType::Time64(_)
            | DataType::Duration(_)
    );
    if is_temporal {
        if let Ok(casted) = cast(arr, &DataType::Utf8) {
            if let Some(a) = casted.as_any().downcast_ref::<StringArray>() {
                if a.is_valid(row) {
                    return a.value(row).to_string();
                }
            }
        }
    }
    arrow::util::display::array_value_to_string(arr, row).unwrap_or_else(|_| "?".to_string())
}

pub fn load(path: &str) -> Result<(SchemaRef, Vec<RecordBatch>)> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let schema = builder.schema().clone();
    let reader = builder.build()?;
    let batches = reader.collect::<Result<Vec<_>, _>>()?;
    Ok((schema, batches))
}
