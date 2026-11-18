//! Stage one COTAHIST member on disk and publish immutable turnover partitions.

use crate::data::schema::turnover_schema;
use crate::ingest::cotahist_parser::CotahistRecord;
use crate::services::cotahist::valid_isin;
use anyhow::{ensure, Context, Result};
use arrow::array::{ArrayRef, Date32Array, RecordBatch, StringArray, UInt64Array};
use chrono::{DateTime, Datelike};
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const BATCH_SIZE: usize = 8192;

pub(super) struct TurnoverStage {
    db: Connection,
}

impl TurnoverStage {
    pub(super) fn new() -> Result<Self> {
        // An empty SQLite filename creates a private temporary database.
        let db = Connection::open("")?;
        db.execute_batch(
            "CREATE TABLE stage (
                year INTEGER NOT NULL, date INTEGER NOT NULL, isin TEXT NOT NULL,
                ticker TEXT NOT NULL, cents INTEGER NOT NULL,
                member TEXT NOT NULL, source_line INTEGER NOT NULL, previous INTEGER NOT NULL,
                PRIMARY KEY(year, isin, date)
            ) WITHOUT ROWID;
            BEGIN;",
        )?;
        Ok(Self { db })
    }

    pub(super) fn push(&self, record: &CotahistRecord, member: &str) -> Result<()> {
        let (Some(cents), Some(isin)) = (record.turnover_cents, record.isin.as_deref()) else {
            return Ok(());
        };
        if record.market_code != "010"
            || record.bdi_code.as_deref() != Some("02")
            || record.currency_code.as_deref() != Some("BRL")
            || !record
                .security_specification
                .as_deref()
                .is_some_and(|kind| {
                    kind.starts_with("ON") || kind.starts_with("PN") || kind.starts_with("UNT")
                })
            || record.raw_ticker.is_empty()
            || !valid_isin(isin)
        {
            return Ok(());
        }
        let date =
            record.date.num_days_from_ce() - DateTime::UNIX_EPOCH.date_naive().num_days_from_ce();
        let inserted = self
            .db
            .prepare_cached(
                "INSERT INTO stage VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)
             ON CONFLICT(year, isin, date) DO UPDATE SET cents=excluded.cents
             WHERE stage.cents=excluded.cents AND stage.ticker=excluded.ticker",
            )?
            .execute(params![
                record.year,
                date,
                isin,
                record.raw_ticker,
                cents,
                member,
                record.source_line
            ])?;
        ensure!(
            inserted != 0,
            "VOLTOT conflict for {isin} on {} in {member}:{}",
            record.date,
            record.source_line
        );
        Ok(())
    }

    pub(super) fn publish(&self, data_dir: &Path) -> Result<bool> {
        let years = self
            .db
            .prepare("SELECT DISTINCT year FROM stage ORDER BY year")?
            .query_map([], |row| row.get::<_, i32>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if years.is_empty() {
            return Ok(false);
        }

        let output = data_dir.join("processed/turnover-v1");
        fs::create_dir_all(&output)?;
        let mut ledger = Connection::open(data_dir.join("ledger/tucano.db"))?;
        ledger.busy_timeout(std::time::Duration::from_secs(30))?;
        let tx = ledger.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS turnover_partitions (
                year INTEGER PRIMARY KEY, filename TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS turnover_snapshot (
                id INTEGER PRIMARY KEY CHECK (id = 1), revision TEXT NOT NULL
             );",
        )?;

        let revision = Uuid::new_v4();
        let mut changed = false;
        // Keep unpublished files under RAII ownership until the ledger commits.
        let mut partitions = Vec::new();
        for year in years {
            let previous: Option<String> = tx
                .query_row(
                    "SELECT filename FROM turnover_partitions WHERE year=?1",
                    [year],
                    |row| row.get(0),
                )
                .optional()?;
            if let Some(previous) = previous {
                self.merge_previous(year, &output.join(previous))?;
            }
            let added: i64 = self.db.query_row(
                "SELECT COUNT(*) FROM stage WHERE year=?1 AND previous=0",
                [year],
                |row| row.get(0),
            )?;
            if added == 0 {
                continue;
            }
            let filename = format!("turnover-{year}-{revision}.parquet");
            let path = output.join(&filename);
            let file = tempfile::NamedTempFile::new_in(&output)?;
            self.write_year(year, file.as_file())?;
            let published = file.into_temp_path();
            published.persist_noclobber(&path)?;
            partitions.push(tempfile::TempPath::try_from_path(&path)?);
            File::open(&output)?.sync_all()?;
            tx.execute(
                "INSERT INTO turnover_partitions VALUES (?1, ?2)
                 ON CONFLICT(year) DO UPDATE SET filename=excluded.filename",
                params![year, filename],
            )?;
            changed = true;
        }
        if changed {
            tx.execute(
                "INSERT INTO turnover_snapshot VALUES (1, ?1)
                 ON CONFLICT(id) DO UPDATE SET revision=excluded.revision",
                [revision.to_string()],
            )?;
        }
        // The immutable files can be orphaned after a crash. Readers discover only committed paths.
        for path in partitions {
            path.keep()?;
        }
        tx.commit()?;
        Ok(changed)
    }

    fn merge_previous(&self, year: i32, path: &Path) -> Result<()> {
        let mut insert = self.db.prepare_cached(
            "INSERT INTO stage VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)
             ON CONFLICT(year, isin, date) DO UPDATE SET
                member=excluded.member, source_line=excluded.source_line, previous=1
             WHERE stage.cents=excluded.cents AND stage.ticker=excluded.ticker",
        )?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path)?)?
            .with_batch_size(BATCH_SIZE)
            .build()?;
        for batch in reader {
            let batch = batch?;
            let dates = batch
                .column(0)
                .as_any()
                .downcast_ref::<Date32Array>()
                .context("turnover date")?;
            let isins = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("turnover ISIN")?;
            let tickers = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("turnover ticker")?;
            let cents = batch
                .column(3)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .context("turnover cents")?;
            let members = batch
                .column(4)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("turnover member")?;
            let lines = batch
                .column(5)
                .as_any()
                .downcast_ref::<UInt64Array>()
                .context("turnover source line")?;
            for i in 0..batch.num_rows() {
                let count = insert.execute(params![
                    year,
                    dates.value(i),
                    isins.value(i),
                    tickers.value(i),
                    cents.value(i),
                    members.value(i),
                    lines.value(i)
                ])?;
                ensure!(
                    count != 0,
                    "VOLTOT conflict for {} on {} with accepted source {}:{}",
                    isins.value(i),
                    DateTime::UNIX_EPOCH.date_naive()
                        + chrono::Duration::days(i64::from(dates.value(i))),
                    members.value(i),
                    lines.value(i)
                );
            }
        }
        Ok(())
    }

    fn write_year(&self, year: i32, file: &File) -> Result<()> {
        let schema = turnover_schema();
        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::default()))
            .set_max_row_group_size(BATCH_SIZE)
            .build();
        let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;
        let mut stmt = self.db.prepare(
            "SELECT date, isin, ticker, cents, member, source_line FROM stage
             WHERE year=?1 ORDER BY isin, date",
        )?;
        let mut rows = stmt.query([year])?;
        let mut written = 0;
        loop {
            let (mut dates, mut isins, mut tickers, mut cents, mut members, mut lines) = (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
            while dates.len() < BATCH_SIZE {
                let Some(row) = rows.next()? else { break };
                dates.push(row.get::<_, i32>(0)?);
                isins.push(row.get::<_, String>(1)?);
                tickers.push(row.get::<_, String>(2)?);
                cents.push(row.get::<_, u64>(3)?);
                members.push(row.get::<_, String>(4)?);
                lines.push(row.get::<_, u64>(5)?);
            }
            if dates.is_empty() {
                break;
            }
            written += dates.len() as i64;
            let columns: Vec<ArrayRef> = vec![
                Arc::new(Date32Array::from(dates)),
                Arc::new(StringArray::from(isins)),
                Arc::new(StringArray::from(tickers)),
                Arc::new(UInt64Array::from(cents)),
                Arc::new(StringArray::from(members)),
                Arc::new(UInt64Array::from(lines)),
            ];
            writer.write(&RecordBatch::try_new(schema.clone(), columns)?)?;
        }
        writer.close()?;
        file.sync_all()?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file.try_clone()?)?;
        ensure!(
            reader.schema() == &schema,
            "turnover partition schema mismatch"
        );
        ensure!(
            reader.metadata().file_metadata().num_rows() == written,
            "turnover partition row count mismatch"
        );
        Ok(())
    }
}
