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
use arrow::compute::filter_record_batch;
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::{WriterProperties, WriterVersion};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Rows buffered per Arrow batch while streaming. Keeps peak RAM bounded (~15-20 MB).
const WRITE_CHUNK_ROWS: usize = 50_000;

/// Maximum rows buffered inside ArrowWriter before flushing a compressed row group to disk.
const MAX_ROW_GROUP_ROWS: usize = 100_000;

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

trait AsCotahistRecord {
    fn as_record(&self) -> &CotahistRecord;
}

impl AsCotahistRecord for CotahistRecord {
    fn as_record(&self) -> &CotahistRecord {
        self
    }
}

impl AsCotahistRecord for &CotahistRecord {
    fn as_record(&self) -> &CotahistRecord {
        self
    }
}

struct YearChunkWriter {
    file_path: PathBuf,
    temp_path: PathBuf,
    writer: Option<ArrowWriter<fs::File>>,
    buffer: Vec<CotahistRecord>,
    written_date_days: HashSet<i32>,
    new_rows: usize,
    committed: bool,
}

impl YearChunkWriter {
    fn new(
        year: i32,
        output_dir: &Path,
        schema: Arc<arrow::datatypes::Schema>,
    ) -> Result<Self, ParquetWriteError> {
        let file_path = output_dir.join(format!("cotahist-{}.parquet", year));
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).map_err(|_| ParquetWriteError::DirCreate {
                path: parent.display().to_string(),
            })?;
        }

        let temp_path = file_path.with_extension("tmp");
        let file = fs::File::create(&temp_path)?;
        let props = WriterProperties::builder()
            .set_compression(parquet::basic::Compression::ZSTD(Default::default()))
            .set_max_row_group_size(MAX_ROW_GROUP_ROWS)
            .set_writer_version(WriterVersion::PARQUET_2_0)
            .build();

        let writer = ArrowWriter::try_new(file, schema, Some(props))?;

        Ok(Self {
            file_path,
            temp_path,
            writer: Some(writer),
            buffer: Vec::with_capacity(WRITE_CHUNK_ROWS),
            written_date_days: HashSet::new(),
            new_rows: 0,
            committed: false,
        })
    }

    fn push(
        &mut self,
        record: CotahistRecord,
        schema: &Arc<arrow::datatypes::Schema>,
    ) -> Result<(), ParquetWriteError> {
        self.buffer.push(record);
        if self.buffer.len() >= WRITE_CHUNK_ROWS {
            self.flush_buffer(schema)?;
        }
        Ok(())
    }

    fn flush_buffer(
        &mut self,
        schema: &Arc<arrow::datatypes::Schema>,
    ) -> Result<(), ParquetWriteError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let batch = records_to_batch(&self.buffer, schema.clone(), &mut self.written_date_days)?;
        if let Some(writer) = self.writer.as_mut() {
            writer.write(&batch)?;
        }
        self.new_rows += self.buffer.len();
        self.buffer.clear();
        Ok(())
    }

    fn write_slice<R: AsCotahistRecord>(
        &mut self,
        records: &[R],
        schema: &Arc<arrow::datatypes::Schema>,
    ) -> Result<(), ParquetWriteError> {
        for chunk in records.chunks(WRITE_CHUNK_ROWS) {
            let batch = records_to_batch(chunk, schema.clone(), &mut self.written_date_days)?;
            if let Some(writer) = self.writer.as_mut() {
                writer.write(&batch)?;
            }
            self.new_rows += chunk.len();
        }
        Ok(())
    }

    fn finish(
        mut self,
        schema: &Arc<arrow::datatypes::Schema>,
    ) -> Result<WriteStats, ParquetWriteError> {
        self.flush_buffer(schema)?;
        self.buffer = Vec::new();

        let mut existing_rows = 0usize;

        // Stream prior year data one batch at a time, excluding any trading dates
        // that are being written in this batch so interrupted retries never duplicate rows.
        if self.file_path.exists() {
            let existing = fs::File::open(&self.file_path)?;
            let builder = ParquetRecordBatchReaderBuilder::try_new(existing)?
                .with_batch_size(WRITE_CHUNK_ROWS);
            let reader = builder.build()?;
            for batch in reader {
                let batch = batch?;
                let date_col = batch.column(3).as_any().downcast_ref::<Date32Array>();

                let has_overlap = date_col.is_some_and(|dates| {
                    (0..dates.len()).any(|i| {
                        !dates.is_null(i) && self.written_date_days.contains(&dates.value(i))
                    })
                });

                if has_overlap {
                    if let Some(dates) = date_col {
                        let mask: BooleanArray = (0..dates.len())
                            .map(|i| {
                                Some(
                                    dates.is_null(i)
                                        || !self.written_date_days.contains(&dates.value(i)),
                                )
                            })
                            .collect();
                        let filtered = filter_record_batch(&batch, &mask)?;
                        if filtered.num_rows() > 0 {
                            existing_rows += filtered.num_rows();
                            if let Some(writer) = self.writer.as_mut() {
                                writer.write(&filtered)?;
                            }
                        }
                    }
                } else {
                    existing_rows += batch.num_rows();
                    if let Some(writer) = self.writer.as_mut() {
                        writer.write(&batch)?;
                    }
                }
            }
        }

        if let Some(writer) = self.writer.take() {
            writer.close()?;
        }

        fs::rename(&self.temp_path, &self.file_path)?;
        self.committed = true;

        let file_size = fs::metadata(&self.file_path)?.len();
        Ok(WriteStats {
            rows_written: existing_rows + self.new_rows,
            file_path: self.file_path.display().to_string(),
            file_size_bytes: file_size,
        })
    }
}

impl Drop for YearChunkWriter {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.writer.take();
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

/// Streaming, year-partitioned Parquet writer that bounds memory to `WRITE_CHUNK_ROWS` per year.
pub struct StreamingParquetWriter {
    output_dir: PathBuf,
    schema: Arc<arrow::datatypes::Schema>,
    writers: HashMap<i32, YearChunkWriter>,
}

impl StreamingParquetWriter {
    /// Create a new streaming writer targeting `output_dir`.
    pub fn new(output_dir: &Path) -> Self {
        Self {
            output_dir: output_dir.to_path_buf(),
            schema: cotahist_schema(),
            writers: HashMap::new(),
        }
    }

    /// Push a single record into its year partition, flushing when the chunk buffer fills.
    pub fn push(&mut self, record: CotahistRecord) -> Result<(), ParquetWriteError> {
        let year = record.year;
        if !self.writers.contains_key(&year) {
            let writer = YearChunkWriter::new(year, &self.output_dir, self.schema.clone())?;
            self.writers.insert(year, writer);
        }
        if let Some(writer) = self.writers.get_mut(&year) {
            writer.push(record, &self.schema)?;
        }
        Ok(())
    }

    /// Flush all buffered chunks, merge non-overlapping existing year rows, and atomically rename files.
    pub fn finish(mut self) -> Result<Vec<WriteStats>, ParquetWriteError> {
        let mut years: Vec<i32> = self.writers.keys().copied().collect();
        years.sort_unstable();

        let mut results = Vec::with_capacity(years.len());
        for year in years {
            if let Some(writer) = self.writers.remove(&year) {
                results.push(writer.finish(&self.schema)?);
            }
        }
        Ok(results)
    }
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
    let schema = cotahist_schema();
    let mut by_year: HashMap<i32, Vec<&CotahistRecord>> = HashMap::new();
    for record in records {
        by_year.entry(record.year).or_default().push(record);
    }

    let mut years: Vec<i32> = by_year.keys().copied().collect();
    years.sort_unstable();

    let mut results = Vec::with_capacity(years.len());
    for year in years {
        if let Some(year_records) = by_year.remove(&year) {
            let mut writer = YearChunkWriter::new(year, output_dir, schema.clone())?;
            writer.write_slice(&year_records, &schema)?;
            results.push(writer.finish(&schema)?);
        }
    }

    Ok(results)
}

fn records_to_batch<R: AsCotahistRecord>(
    records: &[R],
    schema: Arc<arrow::datatypes::Schema>,
    written_date_days: &mut HashSet<i32>,
) -> Result<RecordBatch, ParquetWriteError> {
    let ticker_arr = StringArray::from(
        records
            .iter()
            .map(|r| r.as_record().ticker.as_str())
            .collect::<Vec<_>>(),
    );
    let isin_arr = StringArray::from(
        records
            .iter()
            .map(|r| r.as_record().isin.as_deref())
            .collect::<Vec<_>>(),
    );
    let company_name_arr = StringArray::from(
        records
            .iter()
            .map(|r| r.as_record().company_name.as_deref())
            .collect::<Vec<_>>(),
    );

    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap_or_default();
    let date_days: Vec<i32> = records
        .iter()
        .map(|r| {
            let days = r.as_record().date.signed_duration_since(epoch).num_days() as i32;
            written_date_days.insert(days);
            days
        })
        .collect();
    let date_arr = Date32Array::from(date_days);
    let year_arr = Int32Array::from(
        records
            .iter()
            .map(|r| r.as_record().year)
            .collect::<Vec<_>>(),
    );
    let market_type_arr = UInt8Array::from(
        records
            .iter()
            .map(|r| r.as_record().market_type)
            .collect::<Vec<_>>(),
    );
    let pre_abr_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_abr)
            .collect::<Vec<_>>(),
    );
    let pre_max_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_max)
            .collect::<Vec<_>>(),
    );
    let pre_min_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_min)
            .collect::<Vec<_>>(),
    );
    let pre_med_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_med)
            .collect::<Vec<_>>(),
    );
    let pre_ult_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_ult)
            .collect::<Vec<_>>(),
    );
    let pre_ofe_compra_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_ofe_compra)
            .collect::<Vec<_>>(),
    );
    let pre_ofe_venda_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().pre_ofe_venda)
            .collect::<Vec<_>>(),
    );
    let tot_neg_arr = UInt32Array::from(
        records
            .iter()
            .map(|r| r.as_record().tot_neg)
            .collect::<Vec<_>>(),
    );
    let quat_tot_arr = UInt64Array::from(
        records
            .iter()
            .map(|r| r.as_record().quat_tot)
            .collect::<Vec<_>>(),
    );
    let vol_tot_arr = Float64Array::from(
        records
            .iter()
            .map(|r| r.as_record().vol_tot)
            .collect::<Vec<_>>(),
    );

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

    #[test]
    fn test_streaming_parquet_writer_deduplicates_overlapping_dates_on_merge() {
        let temp = TempDir::new().unwrap();

        // Initial write with day 2 and day 3
        let mut writer1 = StreamingParquetWriter::new(temp.path());
        writer1.push(make_test_record("PETR4", 2026, 2)).unwrap();
        writer1.push(make_test_record("VALE3", 2026, 3)).unwrap();
        let stats1 = writer1.finish().unwrap();
        assert_eq!(stats1[0].rows_written, 2);

        // Second write re-writes day 3 and adds day 4 (simulating retry after crash)
        let mut writer2 = StreamingParquetWriter::new(temp.path());
        writer2.push(make_test_record("VALE3", 2026, 3)).unwrap();
        writer2.push(make_test_record("ITUB4", 2026, 4)).unwrap();
        let stats2 = writer2.finish().unwrap();

        // Day 2 (existing) + Day 3 (replaced, not duplicated) + Day 4 (new) = 3 rows
        assert_eq!(stats2[0].rows_written, 3);
    }
}
