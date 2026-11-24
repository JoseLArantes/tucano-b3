//! SQLite ledger connection and operations.

use chrono::NaiveDate;
use rusqlite::{
    params_from_iter, types::Value, Connection, OptionalExtension, TransactionBehavior,
};
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

/// Provento draft stored with exact BRL micro-units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProventoDraft {
    pub ticker: String,
    pub tipo: String,
    pub valor_micros: i64,
    pub data_com: Option<String>,
    pub data_pagamento: Option<String>,
    pub obs: Option<String>,
}

/// Stored provento row returned from the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProventoEntry {
    pub id: i64,
    pub ticker: String,
    pub tipo: String,
    pub valor_micros: i64,
    pub data_com: Option<String>,
    pub data_pagamento: Option<String>,
    pub obs: Option<String>,
    pub criado_em: String,
    pub atualizado_em: String,
}

/// Filters shared by paged provento queries.
#[derive(Debug, Clone, Default)]
pub struct ProventoFilters {
    pub tickers: Vec<String>,
    pub tipo: Option<String>,
    pub data_pagamento_desde: Option<String>,
    pub data_pagamento_ate: Option<String>,
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
            );
            CREATE TABLE IF NOT EXISTS market_calendars (
                year INTEGER PRIMARY KEY,
                payload TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS market_calendar_seeds (
                seed_id TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS proventos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ticker TEXT NOT NULL CHECK(length(ticker) BETWEEN 1 AND 10),
                tipo TEXT NOT NULL CHECK(tipo IN ('Dividendo', 'JCP', 'Rendimento')),
                valor_micros INTEGER NOT NULL CHECK(valor_micros BETWEEN 1 AND 99999999999999),
                data_com TEXT,
                data_pagamento TEXT,
                obs TEXT CHECK(obs IS NULL OR length(obs) <= 255),
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_proventos_ticker ON proventos(ticker);
            CREATE INDEX IF NOT EXISTS idx_proventos_ticker_data_com ON proventos(ticker, data_com);
            CREATE INDEX IF NOT EXISTS idx_proventos_pagamento ON proventos(data_pagamento);
            CREATE TABLE IF NOT EXISTS proventos_seeds (
                seed_id TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL
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

    /// List years with a stored B3 market calendar in ascending order.
    pub fn list_market_calendar_years(&self) -> Result<Vec<i32>, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare("SELECT year FROM market_calendars ORDER BY year ASC")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(LedgerError::from)
    }

    /// Read a stored market calendar JSON document and its last update time.
    pub fn get_market_calendar(&self, year: i32) -> Result<Option<(String, String)>, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        conn.query_row(
            "SELECT payload, updated_at FROM market_calendars WHERE year = ?1",
            [year],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(LedgerError::from)
    }

    /// Insert a calendar if the year has no record yet. Returns false on conflict.
    pub fn insert_market_calendar_if_absent(
        &self,
        year: i32,
        payload: &str,
        updated_at: &str,
    ) -> Result<bool, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let inserted = conn.execute(
            "INSERT OR IGNORE INTO market_calendars (year, payload, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![year, payload, updated_at],
        )?;
        Ok(inserted == 1)
    }

    /// Apply a built-in calendar seed once for this ledger, even if that year is deleted later.
    pub fn initialize_market_calendar_seed(
        &self,
        seed_id: &str,
        year: i32,
        payload: &str,
        applied_at: &str,
    ) -> Result<(), LedgerError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let tx = conn.transaction()?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO market_calendar_seeds (seed_id, applied_at) VALUES (?1, ?2)",
            rusqlite::params![seed_id, applied_at],
        )?;
        if inserted == 1 {
            tx.execute(
                "INSERT OR IGNORE INTO market_calendars (year, payload, updated_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![year, payload, applied_at],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Insert a calendar, returning false if the year already exists.
    pub fn create_market_calendar(
        &self,
        year: i32,
        payload: &str,
        updated_at: &str,
    ) -> Result<bool, LedgerError> {
        self.insert_market_calendar_if_absent(year, payload, updated_at)
    }

    /// Replace an existing calendar. Returns false if the year is not stored.
    pub fn update_market_calendar(
        &self,
        year: i32,
        payload: &str,
        updated_at: &str,
    ) -> Result<bool, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let updated = conn.execute(
            "UPDATE market_calendars SET payload = ?2, updated_at = ?3 WHERE year = ?1",
            rusqlite::params![year, payload, updated_at],
        )?;
        Ok(updated == 1)
    }

    /// Delete a stored calendar. Returns false if the year is not stored.
    pub fn delete_market_calendar(&self, year: i32) -> Result<bool, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let deleted = conn.execute("DELETE FROM market_calendars WHERE year = ?1", [year])?;
        Ok(deleted == 1)
    }

    /// Apply a built-in provento seed exactly once and atomically with its rows.
    pub fn initialize_proventos_seed(
        &self,
        seed_id: &str,
        records: &[ProventoDraft],
        applied_at: &str,
    ) -> Result<(), LedgerError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let tx = conn.transaction()?;
        let inserted = tx.execute(
            "INSERT OR IGNORE INTO proventos_seeds (seed_id, applied_at) VALUES (?1, ?2)",
            rusqlite::params![seed_id, applied_at],
        )?;
        if inserted == 1 {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO proventos
                    (ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            )?;
            for record in records {
                stmt.execute(rusqlite::params![
                    record.ticker,
                    record.tipo,
                    record.valor_micros,
                    record.data_com,
                    record.data_pagamento,
                    record.obs,
                    applied_at,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Import new normalized records in one transaction, skipping exact matches.
    pub fn import_proventos_csv(
        &self,
        records: &[ProventoDraft],
        now: &str,
    ) -> Result<usize, LedgerError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut inserted = 0;
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO proventos
                    (ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at)
                 SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7
                 WHERE NOT EXISTS (
                     SELECT 1 FROM proventos
                     WHERE ticker = ?1 AND tipo = ?2 AND valor_micros = ?3
                       AND data_com IS ?4 AND data_pagamento IS ?5 AND obs IS ?6
                 )",
            )?;
            for record in records {
                inserted += stmt.execute(rusqlite::params![
                    record.ticker,
                    record.tipo,
                    record.valor_micros,
                    record.data_com,
                    record.data_pagamento,
                    record.obs,
                    now,
                ])?;
            }
        }
        tx.commit()?;
        Ok(inserted)
    }

    /// Insert one provento and return the persisted row.
    pub fn create_provento(
        &self,
        record: &ProventoDraft,
        now: &str,
    ) -> Result<ProventoEntry, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        conn.execute(
            "INSERT INTO proventos
                (ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            rusqlite::params![
                record.ticker,
                record.tipo,
                record.valor_micros,
                record.data_com,
                record.data_pagamento,
                record.obs,
                now,
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.query_row(
            "SELECT id, ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at
             FROM proventos WHERE id = ?1",
            [id],
            map_provento_row,
        )
        .map_err(LedgerError::from)
    }

    /// Return a stored provento by its stable identifier.
    pub fn get_provento(&self, id: i64) -> Result<Option<ProventoEntry>, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        conn.query_row(
            "SELECT id, ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at
             FROM proventos WHERE id = ?1",
            [id],
            map_provento_row,
        )
        .optional()
        .map_err(LedgerError::from)
    }

    /// Return a deterministic page and the total count matching the supplied filters.
    pub fn list_proventos(
        &self,
        filters: &ProventoFilters,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<ProventoEntry>, usize), LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let mut conditions = Vec::new();
        let mut filter_values = Vec::<Value>::new();

        if !filters.tickers.is_empty() {
            let placeholders = (1..=filters.tickers.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(", ");
            filter_values.extend(filters.tickers.iter().cloned().map(Value::Text));
            conditions.push(format!("ticker IN ({placeholders})"));
        }
        if let Some(tipo) = &filters.tipo {
            filter_values.push(Value::Text(tipo.clone()));
            conditions.push(format!("tipo = ?{}", filter_values.len()));
        }
        if let Some(desde) = &filters.data_pagamento_desde {
            filter_values.push(Value::Text(desde.clone()));
            conditions.push(format!("data_pagamento >= ?{}", filter_values.len()));
        }
        if let Some(ate) = &filters.data_pagamento_ate {
            filter_values.push(Value::Text(ate.clone()));
            conditions.push(format!("data_pagamento <= ?{}", filter_values.len()));
        }

        let where_clause = if conditions.is_empty() {
            "1 = 1".to_string()
        } else {
            conditions.join(" AND ")
        };
        let total_sql = format!("SELECT COUNT(*) FROM proventos WHERE {where_clause}");
        let total: i64 =
            conn.query_row(&total_sql, params_from_iter(filter_values.iter()), |row| {
                row.get(0)
            })?;

        let list_sql = format!(
            "SELECT id, ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at
             FROM proventos
             WHERE {where_clause}
             ORDER BY data_pagamento IS NULL ASC, data_pagamento ASC, ticker ASC, tipo ASC, id ASC
             LIMIT ?{} OFFSET ?{}",
            filter_values.len() + 1,
            filter_values.len() + 2,
        );
        let mut list_values = filter_values;
        list_values.push(Value::Integer(i64::from(limit)));
        list_values.push(Value::Integer(i64::from(offset)));
        let mut stmt = conn.prepare(&list_sql)?;
        let rows = stmt.query_map(params_from_iter(list_values.iter()), map_provento_row)?;
        let entries = rows.collect::<Result<Vec<_>, _>>()?;
        Ok((entries, total as usize))
    }

    /// Sum the stored unit values per ticker over an inclusive data-com interval.
    pub fn proventos_period_totals(
        &self,
        tickers: &[String],
        desde: NaiveDate,
        ate: NaiveDate,
    ) -> Result<Vec<(String, i128)>, LedgerError> {
        self.query_proventos_report(tickers, desde, ate, "ticker", |row| {
            Ok((row.get(0)?, report_micros(row, 1)?))
        })
    }

    /// Sum the stored unit values per ticker and data-com, in chronological order for each ticker.
    pub fn proventos_daily_totals(
        &self,
        tickers: &[String],
        desde: NaiveDate,
        ate: NaiveDate,
    ) -> Result<Vec<(String, NaiveDate, i128)>, LedgerError> {
        self.query_proventos_report(tickers, desde, ate, "ticker, data_com", |row| {
            Ok((row.get(0)?, row.get(1)?, report_micros(row, 2)?))
        })
    }

    fn query_proventos_report<T>(
        &self,
        tickers: &[String],
        desde: NaiveDate,
        ate: NaiveDate,
        group_by: &str,
        map: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, LedgerError> {
        if tickers.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; tickers.len()].join(", ");
        // Sum whole reais and fractional micro-units separately to keep both integer
        // aggregates in range even when the combined amount exceeds i64 micro-units.
        let sql = format!(
            "SELECT {group_by}, SUM(valor_micros / 1000000), SUM(valor_micros % 1000000)
             FROM proventos
             WHERE data_com BETWEEN ? AND ? AND ticker IN ({placeholders})
             GROUP BY {group_by} ORDER BY {group_by}"
        );
        let dates: [&dyn rusqlite::ToSql; 2] = [&desde, &ate];
        let params = dates
            .into_iter()
            .chain(tickers.iter().map(|ticker| ticker as &dyn rusqlite::ToSql));

        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), map)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Replace a provento's business fields, preserving its creation timestamp.
    pub fn update_provento(
        &self,
        id: i64,
        record: &ProventoDraft,
        now: &str,
    ) -> Result<Option<ProventoEntry>, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let updated = conn.execute(
            "UPDATE proventos SET ticker = ?2, tipo = ?3, valor_micros = ?4,
                 data_com = ?5, data_pagamento = ?6, obs = ?7, updated_at = ?8
             WHERE id = ?1",
            rusqlite::params![
                id,
                record.ticker,
                record.tipo,
                record.valor_micros,
                record.data_com,
                record.data_pagamento,
                record.obs,
                now,
            ],
        )?;
        if updated == 0 {
            return Ok(None);
        }
        conn.query_row(
            "SELECT id, ticker, tipo, valor_micros, data_com, data_pagamento, obs, created_at, updated_at
             FROM proventos WHERE id = ?1",
            [id],
            map_provento_row,
        )
        .map(Some)
        .map_err(LedgerError::from)
    }

    /// Delete a provento. Returns false when the identifier does not exist.
    pub fn delete_provento(&self, id: i64) -> Result<bool, LedgerError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| LedgerError::Lock(e.to_string()))?;
        let deleted = conn.execute("DELETE FROM proventos WHERE id = ?1", [id])?;
        Ok(deleted == 1)
    }
}

fn report_micros(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<i128> {
    let reais = i128::from(row.get::<_, i64>(offset)?);
    let micros = i128::from(row.get::<_, i64>(offset + 1)?);
    Ok(reais * 1_000_000 + micros)
}

fn map_provento_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProventoEntry> {
    Ok(ProventoEntry {
        id: row.get(0)?,
        ticker: row.get(1)?,
        tipo: row.get(2)?,
        valor_micros: row.get(3)?,
        data_com: row.get(4)?,
        data_pagamento: row.get(5)?,
        obs: row.get(6)?,
        criado_em: row.get(7)?,
        atualizado_em: row.get(8)?,
    })
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
