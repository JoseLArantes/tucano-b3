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
use crate::ingest::cotahist_parser::CotahistParser;
use crate::ingest::parquet_writer::{self, StreamingParquetWriter};
use crate::ingest::zip_handler::{self, MAX_ZIP_SIZE};
use crate::ledger::{Ledger, TickerBatchEntry};
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;
use zip::ZipArchive;

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
    /// Ingest from uploaded ZIP data in memory.
    ///
    /// Streams each archive entry directly through the chunked parser and Parquet writer
    /// without materializing uncompressed `.TXT` buffers or full-year record vectors.
    pub fn ingest_zip(
        data: Vec<u8>,
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        let size = data.len();
        let cursor = std::io::Cursor::new(data);
        Self::ingest_zip_reader(cursor, size, data_dir, data_loader)
    }

    /// Ingest from a ZIP file on disk without loading the archive into RAM.
    pub fn ingest_zip_path(
        zip_path: &Path,
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        let metadata = std::fs::metadata(zip_path)?;
        let size = metadata.len() as usize;
        let file = std::fs::File::open(zip_path)?;
        Self::ingest_zip_reader(file, size, data_dir, data_loader)
    }

    /// Ingest from any seekable ZIP reader.
    pub fn ingest_zip_reader<R: Read + Seek>(
        reader: R,
        size: usize,
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        if size > MAX_ZIP_SIZE {
            return Err(IngestError::Zip(zip_handler::ZipError::FileTooLarge {
                size,
                max: MAX_ZIP_SIZE,
            }));
        }

        let task_id = Uuid::new_v4().to_string();
        tracing::info!(task_id = %task_id, "Iniciando ingestão de ZIP");

        let mut archive = ZipArchive::new(reader).map_err(zip_handler::ZipError::from)?;
        let mut matching_indices = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(zip_handler::ZipError::from)?;
            if let Some(filename) = zip_handler::cotahist_entry_filename(file.name()) {
                matching_indices.push((i, filename));
            }
        }

        if matching_indices.is_empty() {
            return Err(IngestError::Zip(zip_handler::ZipError::NoCotahistFiles));
        }

        tracing::info!(
            count = matching_indices.len(),
            "Arquivos COTAHIST extraídos do ZIP"
        );

        let mut state = IngestRunState::new(&task_id, data_dir)?;
        for (idx, filename) in matching_indices {
            let entry = archive.by_index(idx).map_err(zip_handler::ZipError::from)?;
            Self::ingest_single_reader(&task_id, &filename, entry, &mut state)?;
        }

        state.finalize(data_loader)
    }

    /// Ingest from a local folder, streaming `.TXT` and `.ZIP` files one by one.
    pub fn ingest_folder(
        folder: &Path,
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, IngestError> {
        let task_id = Uuid::new_v4().to_string();
        tracing::info!(task_id = %task_id, folder = %folder.display(), "Iniciando ingestão de pasta");

        let files = zip_handler::scan_folder_for_cotahist(folder)?;
        let mut state = IngestRunState::new(&task_id, data_dir)?;

        for path in &files {
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if filename.to_uppercase().ends_with(".ZIP") {
                if let Ok(file) = std::fs::File::open(path) {
                    if let Ok(mut archive) = ZipArchive::new(file) {
                        for i in 0..archive.len() {
                            let entry_name = match archive.by_index(i) {
                                Ok(e) => zip_handler::cotahist_entry_filename(e.name()),
                                Err(_) => None,
                            };
                            if let Some(inner_name) = entry_name {
                                if let Ok(entry) = archive.by_index(i) {
                                    Self::ingest_single_reader(
                                        &task_id,
                                        &inner_name,
                                        entry,
                                        &mut state,
                                    )?;
                                }
                            }
                        }
                    }
                }
            } else if let Ok(file) = std::fs::File::open(path) {
                Self::ingest_single_reader(&task_id, &filename, file, &mut state)?;
            }
        }

        state.finalize(data_loader)
    }

    /// Stream-parse a single COTAHIST `.TXT` reader, write Parquet in bounded chunks,
    /// and commit new dates and ticker metadata in a single SQLite transaction.
    fn ingest_single_reader<R: Read>(
        task_id: &str,
        filename: &str,
        reader: R,
        state: &mut IngestRunState,
    ) -> Result<(), IngestError> {
        tracing::info!(
            task_id = %task_id,
            file = %filename,
            "Processando arquivo"
        );

        let mut parquet_writer = StreamingParquetWriter::new(&state.output_dir);
        let mut new_dates: HashSet<NaiveDate> = HashSet::new();
        let mut unique_tickers: HashMap<String, TickerBatchEntry> = HashMap::new();
        let mut records_new = 0usize;

        let stats = CotahistParser::parse_stream(reader, |record| -> Result<(), IngestError> {
            // Track ticker metadata across all parsed rows (allows self-healing if a prior run
            // crashed after inserting dates but before committing tickers).
            if let Some(entry) = unique_tickers.get_mut(&record.ticker) {
                if record.date < entry.first_seen {
                    entry.first_seen = record.date;
                }
                if record.date >= entry.last_seen {
                    entry.last_seen = record.date;
                    if record.isin.is_some() {
                        entry.isin.clone_from(&record.isin);
                    }
                    if record.company_name.is_some() {
                        entry.company_name.clone_from(&record.company_name);
                    }
                } else {
                    if entry.isin.is_none() && record.isin.is_some() {
                        entry.isin.clone_from(&record.isin);
                    }
                    if entry.company_name.is_none() && record.company_name.is_some() {
                        entry.company_name.clone_from(&record.company_name);
                    }
                }
            } else {
                unique_tickers.insert(
                    record.ticker.clone(),
                    TickerBatchEntry {
                        ticker: record.ticker.clone(),
                        isin: record.isin.clone(),
                        company_name: record.company_name.clone(),
                        first_seen: record.date,
                        last_seen: record.date,
                    },
                );
            }

            if !state.ingested_set.contains(&record.date) {
                records_new += 1;
                new_dates.insert(record.date);
                parquet_writer.push(record)?;
            }

            Ok(())
        })?;

        state.ingested_set.extend(new_dates.iter().copied());

        let records_parsed = stats.lines_parsed;
        let records_skipped = records_parsed.saturating_sub(records_new);

        tracing::info!(
            task_id = %task_id,
            file = %filename,
            parsed = records_parsed,
            new = records_new,
            skipped = records_skipped,
            "Arquivo processado"
        );

        let mut file_result = FileIngestResult {
            filename: filename.to_string(),
            records_parsed,
            records_new,
            records_skipped,
            date_min: stats.date_min,
            date_max: stats.date_max,
            parquet_files: Vec::new(),
        };

        if records_new > 0 {
            let write_stats = parquet_writer.finish()?;
            tracing::info!(
                task_id = %task_id,
                file = %filename,
                records = records_new,
                files = write_stats.len(),
                "Parquet gravado"
            );
            file_result.parquet_files = write_stats;
            state.wrote_any = true;
        }

        if !new_dates.is_empty() || !unique_tickers.is_empty() {
            let ticker_entries: Vec<TickerBatchEntry> = unique_tickers.into_values().collect();
            state
                .ledger
                .record_ingestion_batch("cotahist", "ingested", &new_dates, &ticker_entries)
                .map_err(|e| IngestError::Ledger(e.to_string()))?;
            state.wrote_any = true;
        }

        state.result.file_results.push(file_result);
        state.result.files_processed += 1;
        state.result.total_records_parsed += records_parsed;
        state.result.total_records_new += records_new;
        state.result.total_records_skipped += records_skipped;

        Ok(())
    }
}

struct IngestRunState {
    output_dir: std::path::PathBuf,
    ledger: Ledger,
    ingested_set: HashSet<NaiveDate>,
    result: IngestResult,
    wrote_any: bool,
}

impl IngestRunState {
    fn new(task_id: &str, data_dir: &Path) -> Result<Self, IngestError> {
        let output_dir = data_dir.join("processed/cotahist");
        let ledger_path = data_dir.join("ledger/tucano.db");

        let ledger = Ledger::open(&ledger_path).map_err(|e| IngestError::Ledger(e.to_string()))?;
        let ingested_dates = ledger
            .get_all_ingested_dates("cotahist")
            .map_err(|e| IngestError::Ledger(e.to_string()))?;
        let ingested_set: HashSet<NaiveDate> = ingested_dates.into_iter().collect();

        Ok(Self {
            output_dir,
            ledger,
            ingested_set,
            result: IngestResult {
                task_id: task_id.to_string(),
                files_processed: 0,
                total_records_parsed: 0,
                total_records_new: 0,
                total_records_skipped: 0,
                file_results: Vec::new(),
                success: true,
                error: None,
            },
            wrote_any: false,
        })
    }

    fn finalize(self, data_loader: &Arc<DataLoader>) -> Result<IngestResult, IngestError> {
        if self.wrote_any {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let loader = Arc::clone(data_loader);
                handle.spawn(async move {
                    if let Err(e) = loader.reload().await {
                        tracing::error!(error = %e, "Erro ao recarregar dados");
                    }
                });
            }
        }

        tracing::info!(
            task_id = %self.result.task_id,
            files = self.result.files_processed,
            parsed = self.result.total_records_parsed,
            new = self.result.total_records_new,
            skipped = self.result.total_records_skipped,
            "Ingestão concluída"
        );

        Ok(self.result)
    }
}
