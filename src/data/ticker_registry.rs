//! Ticker registry for tucano-b3.
//!
//! Loads ticker metadata from the SQLite ledger produced by tucano-b3-cli.
//! Provides O(1) ticker existence checks for API validation.

use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use thiserror::Error;

/// Ticker metadata loaded from the ledger.
#[derive(Debug, Clone)]
pub struct TickerInfo {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_active: bool,
}

/// Errors during ticker registry operations.
#[derive(Error, Debug)]
pub enum TickerRegistryError {
    #[error("Ledger file not found: {path}")]
    LedgerNotFound { path: String },

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Failed to open ledger: {path}")]
    OpenFailed { path: String },

    #[error("Internal synchronization error: {0}")]
    LockPoisoned(String),
}

/// Thread-safe ticker registry.
///
/// Loaded once at startup from the SQLite ledger.
/// Provides O(1) ticker existence checks.
pub struct TickerRegistry {
    tickers: RwLock<HashMap<String, TickerInfo>>,
}

impl TickerRegistry {
    /// Load the ticker registry from the SQLite ledger.
    ///
    /// Reads the `tickers` table from `data/ledger/tucano.db`.
    /// If the ledger doesn't exist, returns an empty registry.
    pub fn load(ledger_path: &Path) -> Result<Self, TickerRegistryError> {
        if !ledger_path.exists() {
            tracing::warn!(
                path = %ledger_path.display(),
                "Ledger não encontrado, registry de tickers vazio"
            );
            return Ok(Self {
                tickers: RwLock::new(HashMap::new()),
            });
        }

        let conn = Connection::open_with_flags(ledger_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| TickerRegistryError::OpenFailed {
                path: ledger_path.display().to_string(),
            })?;

        let mut stmt = conn.prepare(
            "SELECT ticker, isin, company_name, first_seen, last_seen, is_active FROM tickers",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(TickerInfo {
                ticker: row.get(0)?,
                isin: row.get(1)?,
                company_name: row.get(2)?,
                first_seen: row.get(3)?,
                last_seen: row.get(4)?,
                is_active: row.get::<_, i32>(5)? != 0,
            })
        })?;

        let mut tickers = HashMap::new();
        for row in rows {
            let info = row?;
            tickers.insert(info.ticker.clone(), info);
        }

        tracing::info!(count = tickers.len(), "Registry de tickers carregado");

        Ok(Self {
            tickers: RwLock::new(tickers),
        })
    }

    /// Check if a ticker exists in the registry.
    pub fn exists(&self, ticker: &str) -> bool {
        let tickers = self
            .tickers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tickers.contains_key(&ticker.to_uppercase())
    }

    /// Get ticker info by name.
    pub fn get(&self, ticker: &str) -> Option<TickerInfo> {
        let tickers = self
            .tickers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tickers.get(&ticker.to_uppercase()).cloned()
    }

    /// Get all tickers, optionally filtered by active status.
    pub fn list(&self, active_only: bool) -> Vec<TickerInfo> {
        let tickers = self
            .tickers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tickers
            .values()
            .filter(|t| !active_only || t.is_active)
            .cloned()
            .collect()
    }

    /// Get the total number of tickers.
    pub fn count(&self) -> usize {
        let tickers = self
            .tickers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        tickers.len()
    }

    /// Reload the registry from the ledger.
    ///
    /// Used by the data reload mechanism (M1.6).
    pub fn reload(&self, ledger_path: &Path) -> Result<(), TickerRegistryError> {
        let new_registry = Self::load(ledger_path)?;
        let new_tickers = new_registry
            .tickers
            .into_inner()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let mut tickers = self
            .tickers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *tickers = new_tickers;

        tracing::info!("Registry de tickers recarregado");
        Ok(())
    }
}
