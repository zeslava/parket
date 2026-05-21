use anyhow::Result;
use arrow::array::RecordBatch;
use arrow::datatypes::SchemaRef;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;

pub fn load(path: &str) -> Result<(SchemaRef, Vec<RecordBatch>)> {
    let file = File::open(path)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let schema = builder.schema().clone();
    let reader = builder.build()?;
    let batches = reader.collect::<Result<Vec<_>, _>>()?;
    Ok((schema, batches))
}
