//! Ingestion service.
//!
//! Orchestrates the full ingestion pipeline:
//! 1. Extract COTAHIST files (from ZIP or folder)
//! 2. Parse each file
//! 3. Filter out already-ingested dates (incremental)
//! 4. Write to Parquet
//! 5. Update SQLite ledger (tickers + ingested dates)
//! 6. Reload DataLoader

use crate::data::reload::DataLoader;
use crate::ingest::cotahist_parser::{CotahistParser, CotahistRecord};
use crate::ingest::parquet_writer;
use crate::ingest::zip_handler::{self, ExtractedFile};
use crate::ledger::Ledger;
use chrono::NaiveDate;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Errors during ingestion.
#[derive(Error, Debug)]
pub enum IngestError {
    #[error("ZIP error: {0}")]
    Zip(#[from] zip_handler::ZipError),

    #[error("Parse error: {0}")]
    Parse(#[from] crate::ingest::cotahist_parser::CotahistParseError),

    #[error("Parquet write error: {0}")]
    ParquetWrite(#[from] parquet_writer::ParquetWriteError),

    #[error("Ledger error: {0}")]
    Ledger(String),

    #[error("Data reload error: {0}")]
    Reload(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of a single file ingestion.
#[derive(Debug, Clone)]
pub struct FileIngestResult {
    pub filename: String,
    pub records_parsed: usize,
    pub records_new: usize,
    pub records_skipped: usize,
    pub date_min: Option<NaiveDate>,
    pub date_max: Option<NaiveDate>,
    pub parquet_files: Vec<parquet_writer::WriteStats>,
}

/// Result of a full ingestion task.
#[derive(Debug, Clone)]
pub struct IngestResult {
    pub task_id: String,
    pub files_processed: usize,
    pub total_records_parsed: usize,
    pub total_records_new: usize,
    pub total_records_skipped: usize,
    pub file_results: Vec<FileIngestResult>,
    pub success: bool,
    pub error: Option<String>,
}

/// Ingestion service.
pub struct IngestService;

impl IngestService {
    /// Ingest from uploaded ZIP data.
    pub fn ingest_zip(
        data: &[u8],
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        let task_id = Uuid::new_v4().to_string();

        tracing::info!(task_id = %task_id, "Iniciando ingestão de ZIP");

        // Extract files from ZIP
        let extracted = zip_handler::extract_cotahist_from_zip(data)?;

        // Process each file
        Self::ingest_extracted_files(&task_id, &extracted, data_dir, data_loader)
    }

    /// Ingest from a local folder.
    pub fn ingest_folder(
        folder: &Path,
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        let task_id = Uuid::new_v4().to_string();

        tracing::info!(task_id = %task_id, folder = %folder.display(), "Iniciando ingestão de pasta");

        // Scan folder
        let files = zip_handler::scan_folder_for_cotahist(folder)?;

        // Read and process each file (support both .TXT and .ZIP archives)
        let mut extracted: Vec<ExtractedFile> = Vec::new();
        for path in &files {
            if let Ok(content) = std::fs::read(path) {
                let filename = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if filename.to_uppercase().ends_with(".ZIP") {
                    if let Ok(files_in_zip) = zip_handler::extract_cotahist_from_zip(&content) {
                        extracted.extend(files_in_zip);
                    }
                } else {
                    extracted.push(ExtractedFile { filename, content });
                }
            }
        }

        Self::ingest_extracted_files(&task_id, &extracted, data_dir, data_loader)
    }

    /// Process extracted COTAHIST files.
    fn ingest_extracted_files(
        task_id: &str,
        files: &[ExtractedFile],
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        let output_dir = data_dir.join("processed/cotahist");
        let ledger_path = data_dir.join("ledger/tucano.db");

        let mut result = IngestResult {
            task_id: task_id.to_string(),
            files_processed: 0,
            total_records_parsed: 0,
            total_records_new: 0,
            total_records_skipped: 0,
            file_results: Vec::new(),
            success: true,
            error: None,
        };

        // Load ledger for incremental check
        let ledger = Ledger::open(&ledger_path).map_err(|e| IngestError::Ledger(e.to_string()))?;

        // Get already-ingested dates
        let ingested_dates = ledger
            .get_all_ingested_dates("cotahist")
            .map_err(|e| IngestError::Ledger(e.to_string()))?;
        let mut ingested_set: HashSet<NaiveDate> = ingested_dates.into_iter().collect();

        let mut all_new_records: Vec<CotahistRecord> = Vec::new();

        for file in files {
            tracing::info!(
                task_id = %task_id,
                file = %file.filename,
                "Processando arquivo"
            );

            // Parse the file
            let cursor = std::io::Cursor::new(&file.content);
            let (records, stats) = CotahistParser::parse(cursor)?;

            // Filter out already-ingested dates (incremental)
            let new_records: Vec<CotahistRecord> = records
                .into_iter()
                .filter(|r| !ingested_set.contains(&r.date))
                .collect();

            // Track newly seen dates within the current batch to avoid duplicate insertions
            for r in &new_records {
                ingested_set.insert(r.date);
            }

            let records_parsed = stats.lines_parsed;
            let records_new = new_records.len();
            let records_skipped = records_parsed.saturating_sub(records_new);

            tracing::info!(
                task_id = %task_id,
                file = %file.filename,
                parsed = records_parsed,
                new = records_new,
                skipped = records_skipped,
                "Arquivo processado"
            );

            let file_result = FileIngestResult {
                filename: file.filename.clone(),
                records_parsed,
                records_new,
                records_skipped,
                date_min: stats.date_min,
                date_max: stats.date_max,
                parquet_files: Vec::new(), // Will be filled after write
            };

            result.file_results.push(file_result);
            result.files_processed += 1;
            result.total_records_parsed += records_parsed;
            result.total_records_new += records_new;
            result.total_records_skipped += records_skipped;

            all_new_records.extend(new_records);
        }

        // Write new records to Parquet
        if !all_new_records.is_empty() {
            let write_stats =
                parquet_writer::write_cotahist_parquet(&all_new_records, &output_dir)?;

            tracing::info!(
                task_id = %task_id,
                records = all_new_records.len(),
                files = write_stats.len(),
                "Parquet gravado"
            );

            // Update file_results with write stats
            for file_res in &mut result.file_results {
                file_res.parquet_files = write_stats.clone();
            }

            // Update ledger with new ingested dates
            let new_dates: HashSet<NaiveDate> = all_new_records.iter().map(|r| r.date).collect();

            for date in &new_dates {
                ledger
                    .insert_ingested_date("cotahist", "ingested", *date)
                    .map_err(|e| IngestError::Ledger(e.to_string()))?;
            }

            // Update ticker registry in ledger (deduplicate in memory to avoid redundant queries)
            use std::collections::HashMap;
            let mut unique_tickers: HashMap<&str, (Option<&str>, Option<&str>, NaiveDate)> =
                HashMap::new();
            for record in &all_new_records {
                unique_tickers
                    .entry(&record.ticker)
                    .and_modify(|e| {
                        if record.date > e.2 {
                            e.2 = record.date;
                            if record.isin.is_some() {
                                e.0 = record.isin.as_deref();
                            }
                            if record.company_name.is_some() {
                                e.1 = record.company_name.as_deref();
                            }
                        }
                    })
                    .or_insert((
                        record.isin.as_deref(),
                        record.company_name.as_deref(),
                        record.date,
                    ));
            }

            for (ticker, (isin, company_name, date)) in unique_tickers {
                ledger
                    .upsert_ticker(ticker, isin, company_name, date)
                    .map_err(|e| IngestError::Ledger(e.to_string()))?;
            }

            // Reload DataLoader to pick up new data
            let rt = tokio::runtime::Handle::try_current();
            if let Ok(handle) = rt {
                let loader = Arc::clone(data_loader);
                handle.spawn(async move {
                    if let Err(e) = loader.reload().await {
                        tracing::error!(error = %e, "Erro ao recarregar dados");
                    }
                });
            }
        }

        tracing::info!(
            task_id = %task_id,
            files = result.files_processed,
            parsed = result.total_records_parsed,
            new = result.total_records_new,
            skipped = result.total_records_skipped,
            "Ingestão concluída"
        );

        Ok(result)
    }
}
