//! Parquet writer for COTAHIST ingestion.
//!
//! Writes parsed COTAHIST records to Parquet files.
//! Output schema matches src/data/schema.rs::cotahist_schema().
//! Uses zstd compression and 1M row groups.
//!
//! Annual files are written in chunks and existing year data is streamed so the
//! process does not hold a full-year Arrow batch plus the record Vec at once.

use crate::data::schema::cotahist_schema;
use crate::ingest::cotahist_parser::CotahistRecord;
use arrow::array::*;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::{WriterProperties, WriterVersion};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Rows per Arrow batch while writing. Keeps peak memory bounded on small nodes.
const WRITE_CHUNK_ROWS: usize = 100_000;

/// Errors during Parquet writing.
#[derive(Error, Debug)]
pub enum ParquetWriteError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),

    #[error("Failed to create directory: {path}")]
    DirCreate { path: String },
}

/// Statistics from a write operation.
#[derive(Debug, Clone)]
pub struct WriteStats {
    pub rows_written: usize,
    pub file_path: String,
    pub file_size_bytes: u64,
}

/// Write COTAHIST records to a Parquet file.
///
/// The output file is partitioned by year:
///   data/processed/cotahist/cotahist-{YYYY}.parquet
///
/// If the file already exists, it is merged with existing records (idempotent).
pub fn write_cotahist_parquet(
    records: &[CotahistRecord],
    output_dir: &Path,
) -> Result<Vec<WriteStats>, ParquetWriteError> {
    // Group record indexes by year to avoid cloning records.
    let mut by_year: std::collections::HashMap<i32, Vec<usize>> = std::collections::HashMap::new();
    for (idx, record) in records.iter().enumerate() {
        by_year.entry(record.year).or_default().push(idx);
    }

    let mut results = Vec::new();

    for (year, indexes) in &by_year {
        let file_path = output_dir.join(format!("cotahist-{}.parquet", year));

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|_| ParquetWriteError::DirCreate {
                path: parent.display().to_string(),
            })?;
        }

        let year_records: Vec<&CotahistRecord> = indexes.iter().map(|&i| &records[i]).collect();
        let stats = write_year_parquet(&year_records, &file_path)?;
        results.push(stats);
    }

    Ok(results)
}

/// Write records for a single year to a Parquet file.
fn write_year_parquet(
    records: &[&CotahistRecord],
    file_path: &Path,
) -> Result<WriteStats, ParquetWriteError> {
    let schema = cotahist_schema();
    let temp_path = file_path.with_extension("tmp");
    let file = fs::File::create(&temp_path)?;
    let props = WriterProperties::builder()
        .set_compression(parquet::basic::Compression::ZSTD(Default::default()))
        .set_max_row_group_size(1_000_000)
        .set_writer_version(WriterVersion::PARQUET_2_0)
        .build();

    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;
    let mut existing_rows = 0usize;

    // Stream prior year data one batch at a time (do not retain all batches).
    if file_path.exists() {
        let existing = fs::File::open(file_path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(existing)?;
        let reader = builder.build()?;
        for batch in reader {
            let batch = batch?;
            existing_rows += batch.num_rows();
            writer.write(&batch)?;
        }
    }

    for chunk in records.chunks(WRITE_CHUNK_ROWS) {
        let batch = records_to_batch(chunk, schema.clone())?;
        writer.write(&batch)?;
    }

    writer.close()?;
    fs::rename(&temp_path, file_path)?;

    let file_size = fs::metadata(file_path)?.len();
    Ok(WriteStats {
        rows_written: existing_rows + records.len(),
        file_path: file_path.display().to_string(),
        file_size_bytes: file_size,
    })
}

fn records_to_batch(
    records: &[&CotahistRecord],
    schema: Arc<arrow::datatypes::Schema>,
) -> Result<RecordBatch, ParquetWriteError> {
    let ticker_arr = StringArray::from(
        records
            .iter()
            .map(|r| r.ticker.as_str())
            .collect::<Vec<_>>(),
    );
    let isin_arr = StringArray::from(
        records
            .iter()
            .map(|r| r.isin.as_deref())
            .collect::<Vec<_>>(),
    );
    let company_name_arr = StringArray::from(
        records
            .iter()
            .map(|r| r.company_name.as_deref())
            .collect::<Vec<_>>(),
    );

    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default();
    let date_arr = Date32Array::from(
        records
            .iter()
            .map(|r| r.date.signed_duration_since(epoch).num_days() as i32)
            .collect::<Vec<_>>(),
    );
    let year_arr = Int32Array::from(records.iter().map(|r| r.year).collect::<Vec<_>>());
    let market_type_arr =
        UInt8Array::from(records.iter().map(|r| r.market_type).collect::<Vec<_>>());
    let pre_abr_arr = Float64Array::from(records.iter().map(|r| r.pre_abr).collect::<Vec<_>>());
    let pre_max_arr = Float64Array::from(records.iter().map(|r| r.pre_max).collect::<Vec<_>>());
    let pre_min_arr = Float64Array::from(records.iter().map(|r| r.pre_min).collect::<Vec<_>>());
    let pre_med_arr = Float64Array::from(records.iter().map(|r| r.pre_med).collect::<Vec<_>>());
    let pre_ult_arr = Float64Array::from(records.iter().map(|r| r.pre_ult).collect::<Vec<_>>());
    let pre_ofe_compra_arr =
        Float64Array::from(records.iter().map(|r| r.pre_ofe_compra).collect::<Vec<_>>());
    let pre_ofe_venda_arr =
        Float64Array::from(records.iter().map(|r| r.pre_ofe_venda).collect::<Vec<_>>());
    let tot_neg_arr = UInt32Array::from(records.iter().map(|r| r.tot_neg).collect::<Vec<_>>());
    let quat_tot_arr = UInt64Array::from(records.iter().map(|r| r.quat_tot).collect::<Vec<_>>());
    let vol_tot_arr = Float64Array::from(records.iter().map(|r| r.vol_tot).collect::<Vec<_>>());

    Ok(RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ticker_arr),
            Arc::new(isin_arr),
            Arc::new(company_name_arr),
            Arc::new(date_arr),
            Arc::new(year_arr),
            Arc::new(market_type_arr),
            arrow::array::new_null_array(schema.field(6).data_type(), records.len()),
            Arc::new(pre_abr_arr),
            Arc::new(pre_max_arr),
            Arc::new(pre_min_arr),
            Arc::new(pre_med_arr),
            Arc::new(pre_ult_arr),
            Arc::new(pre_ofe_compra_arr),
            Arc::new(pre_ofe_venda_arr),
            Arc::new(tot_neg_arr),
            Arc::new(quat_tot_arr),
            Arc::new(vol_tot_arr),
            arrow::array::new_null_array(schema.field(17).data_type(), records.len()),
            arrow::array::new_null_array(schema.field(18).data_type(), records.len()),
            arrow::array::new_null_array(schema.field(19).data_type(), records.len()),
            arrow::array::new_null_array(schema.field(20).data_type(), records.len()),
        ],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::schema::validate_schema;
    use chrono::NaiveDate;
    use tempfile::TempDir;

    fn make_test_record(ticker: &str, year: i32, day: u32) -> CotahistRecord {
        CotahistRecord {
            ticker: ticker.to_string(),
            isin: Some(format!("BR{}001", ticker)),
            company_name: Some(format!("{} S.A.", ticker)),
            date: NaiveDate::from_ymd_opt(year, 1, day).unwrap(),
            year,
            market_type: Some(10),
            pre_abr: Some(10.0),
            pre_max: Some(11.0),
            pre_min: Some(9.5),
            pre_med: Some(10.2),
            pre_ult: Some(10.5),
            pre_ofe_compra: Some(10.4),
            pre_ofe_venda: Some(10.6),
            tot_neg: Some(100),
            quat_tot: Some(10000),
            vol_tot: Some(105000.0),
        }
    }

    #[test]
    fn test_write_cotahist_parquet_creates_file_with_valid_schema() {
        let temp = TempDir::new().unwrap();
        let records = vec![
            make_test_record("PETR4", 2026, 2),
            make_test_record("VALE3", 2026, 2),
        ];

        let stats = write_cotahist_parquet(&records, temp.path()).unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].rows_written, 2);

        let file_path = temp.path().join("cotahist-2026.parquet");
        assert!(file_path.exists());

        let file = fs::File::open(&file_path).unwrap();
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
        let file_schema = builder.schema();
        let expected = cotahist_schema();
        assert!(validate_schema(file_schema, &expected, "cotahist-2026.parquet").is_ok());
    }

    #[test]
    fn test_write_cotahist_parquet_partitioned_by_year() {
        let temp = TempDir::new().unwrap();
        let records = vec![
            make_test_record("PETR4", 2025, 2),
            make_test_record("VALE3", 2026, 2),
        ];

        let stats = write_cotahist_parquet(&records, temp.path()).unwrap();
        assert_eq!(stats.len(), 2);

        assert!(temp.path().join("cotahist-2025.parquet").exists());
        assert!(temp.path().join("cotahist-2026.parquet").exists());
    }

    #[test]
    fn test_write_cotahist_parquet_appends_to_existing_file() {
        let temp = TempDir::new().unwrap();
        let records_day1 = vec![make_test_record("PETR4", 2026, 2)];
        let stats1 = write_cotahist_parquet(&records_day1, temp.path()).unwrap();
        assert_eq!(stats1[0].rows_written, 1);

        let records_day2 = vec![make_test_record("VALE3", 2026, 3)];
        let stats2 = write_cotahist_parquet(&records_day2, temp.path()).unwrap();
        assert_eq!(stats2[0].rows_written, 2);
    }

    #[test]
    fn test_write_cotahist_parquet_chunked_large_input() {
        let temp = TempDir::new().unwrap();
        let records: Vec<_> = (1..=250_000)
            .map(|i| make_test_record("PETR4", 2026, ((i % 28) + 1) as u32))
            .collect();
        let stats = write_cotahist_parquet(&records, temp.path()).unwrap();
        assert_eq!(stats[0].rows_written, 250_000);
    }
}
