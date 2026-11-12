//! SQLite ledger connection and operations.

use chrono::NaiveDate;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;
use thiserror::Error;

/// Aggregated ticker metadata for batch ledger upserts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickerBatchEntry {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: NaiveDate,
    pub last_seen: NaiveDate,
}

/// Errors during ledger operations.
#[derive(Error, Debug)]
pub enum LedgerError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to open ledger: {path}")]
    OpenFailed { path: String },

    #[error("Internal synchronization error: {0}")]
    Lock(String),
}

/// Thread-safe SQLite ledger.
pub struct Ledger {
    conn: Mutex<Connection>,
}

impl Ledger {
    /// Open or create the SQLite ledger file.
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path).map_err(|_| LedgerError::OpenFailed {
            path: path.display().to_string(),
        })?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
            CREATE TABLE IF NOT EXISTS tickers (
                ticker TEXT PRIMARY KEY,
                isin TEXT,
                company_name TEXT,
                first_seen TEXT,
                last_seen TEXT,
                is_active INTEGER
            );
            CREATE TABLE IF NOT EXISTS ingested_dates (
                format TEXT NOT NULL,
                partition_key TEXT NOT NULL,
                date TEXT NOT NULL,
                ingestion_id TEXT,
                PRIMARY KEY (format, date)
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Get all ingested dates for a format.
    pub fn get_all_ingested_dates(&self, format: &str) -> Result<Vec<NaiveDate>, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;

        let mut stmt =
            conn.prepare("SELECT DISTINCT date FROM ingested_dates WHERE format = ?1")?;

        let rows = stmt.query_map(rusqlite::params![format], |row| {
            let date_str: String = row.get(0)?;
            Ok(date_str)
        })?;

        let mut dates = Vec::new();
        for row in rows {
            let date_str = row?;
            if let Ok(date) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                dates.push(date);
            }
        }

        Ok(dates)
    }

    /// Insert an ingested date.
    pub fn insert_ingested_date(
        &self,
        format: &str,
        partition_key: &str,
        date: NaiveDate,
    ) -> Result<(), LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;

        conn.execute(
            "INSERT OR IGNORE INTO ingested_dates (format, partition_key, date, ingestion_id)
             VALUES (?1, ?2, ?3, NULL)",
            rusqlite::params![format, partition_key, date.format("%Y-%m-%d").to_string()],
        )?;
        Ok(())
    }

    /// Upsert a ticker in the registry.
    pub fn upsert_ticker(
        &self,
        ticker: &str,
        isin: Option<&str>,
        company_name: Option<&str>,
        date: NaiveDate,
    ) -> Result<(), LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;

        conn.execute(
            "INSERT INTO tickers (ticker, isin, company_name, first_seen, last_seen, is_active)
             VALUES (?1, ?2, ?3, ?4, ?4, 1)
             ON CONFLICT(ticker) DO UPDATE SET
                 last_seen = MAX(last_seen, ?4),
                 first_seen = MIN(first_seen, ?4),
                 isin = COALESCE(?2, isin),
                 company_name = COALESCE(?3, company_name)",
            rusqlite::params![
                ticker,
                isin,
                company_name,
                date.format("%Y-%m-%d").to_string()
            ],
        )?;
        Ok(())
    }

    /// Atomically record a batch of ingested dates and ticker upserts in a single SQLite transaction.
    ///
    /// Wrapping thousands of ticker upserts and trading dates in one transaction avoids per-row
    /// journal fsync overhead on networked or block-backed PersistentVolumes.
    pub fn record_ingestion_batch(
        &self,
        format: &str,
        partition_key: &str,
        dates: &HashSet<NaiveDate>,
        tickers: &[TickerBatchEntry],
    ) -> Result<(), LedgerError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;

        let tx = conn.transaction()?;
        {
            let mut date_stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO ingested_dates (format, partition_key, date, ingestion_id)
                 VALUES (?1, ?2, ?3, NULL)",
            )?;
            for date in dates {
                let date_str = date.format("%Y-%m-%d").to_string();
                date_stmt.execute(rusqlite::params![format, partition_key, date_str])?;
            }

            let mut ticker_stmt = tx.prepare_cached(
                "INSERT INTO tickers (ticker, isin, company_name, first_seen, last_seen, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1)
                 ON CONFLICT(ticker) DO UPDATE SET
                     first_seen = MIN(first_seen, excluded.first_seen),
                     last_seen = MAX(last_seen, excluded.last_seen),
                     isin = COALESCE(excluded.isin, isin),
                     company_name = COALESCE(excluded.company_name, company_name)",
            )?;
            for entry in tickers {
                let first_str = entry.first_seen.format("%Y-%m-%d").to_string();
                let last_str = entry.last_seen.format("%Y-%m-%d").to_string();
                ticker_stmt.execute(rusqlite::params![
                    entry.ticker,
                    entry.isin,
                    entry.company_name,
                    first_str,
                    last_str,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ledger_ingested_dates_and_tickers() {
        let temp = TempDir::new().unwrap();
        let ledger_path = temp.path().join("ledger/tucano.db");

        let ledger = Ledger::open(&ledger_path).unwrap();

        let date1 = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();

        ledger
            .insert_ingested_date("cotahist", "2026", date1)
            .unwrap();
        // Duplicate insert should be ignored
        ledger
            .insert_ingested_date("cotahist", "2026", date1)
            .unwrap();
        ledger
            .insert_ingested_date("cotahist", "2026", date2)
            .unwrap();

        let dates = ledger.get_all_ingested_dates("cotahist").unwrap();
        assert_eq!(dates.len(), 2);
        assert!(dates.contains(&date1));
        assert!(dates.contains(&date2));

        // Test ticker upsert
        ledger
            .upsert_ticker("PETR4", Some("BRPETR001"), Some("PETROBRAS"), date1)
            .unwrap();
        ledger.upsert_ticker("PETR4", None, None, date2).unwrap();

        let conn = ledger.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT ticker, isin, company_name, first_seen, last_seen FROM tickers WHERE ticker = 'PETR4'",
            )
            .unwrap();
        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .unwrap();

        assert_eq!(row.0, "PETR4");
        assert_eq!(row.1.as_deref(), Some("BRPETR001"));
        assert_eq!(row.2.as_deref(), Some("PETROBRAS"));
        assert_eq!(row.3, "2026-01-02");
        assert_eq!(row.4, "2026-01-03");
    }

    #[test]
    fn test_ledger_batch_ingestion_transaction() {
        let temp = TempDir::new().unwrap();
        let ledger_path = temp.path().join("ledger/tucano.db");
        let ledger = Ledger::open(&ledger_path).unwrap();

        let d1 = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2025, 12, 30).unwrap();
        let dates: HashSet<NaiveDate> = [d1, d2].into_iter().collect();

        let tickers: Vec<TickerBatchEntry> = (0..5_000)
            .map(|i| TickerBatchEntry {
                ticker: format!("TICK{i:04}"),
                isin: Some(format!("BRTICK{i:04}01")),
                company_name: Some(format!("COMPANY {i}")),
                first_seen: d1,
                last_seen: d2,
            })
            .collect();

        ledger
            .record_ingestion_batch("cotahist", "ingested", &dates, &tickers)
            .unwrap();

        let ingested = ledger.get_all_ingested_dates("cotahist").unwrap();
        assert_eq!(ingested.len(), 2);

        let conn = ledger.conn.lock().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tickers", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 5_000);
    }
}
