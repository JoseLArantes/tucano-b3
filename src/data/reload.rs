//! Data reload mechanism for tucano-b3.
//!
//! Supports hot-reloading of Parquet files and ticker registry
//! without restarting the server.

use crate::cache::store::CacheStore;
use crate::config::DataConfig;
use crate::data::query_service::QueryService;
use crate::data::reader::discover_files;
use crate::data::ticker_registry::TickerRegistry;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Errors during data reload.
#[derive(Error, Debug)]
pub enum ReloadError {
    #[error("Failed to discover files: {0}")]
    Discovery(#[from] crate::data::reader::ReaderError),

    #[error("Failed to create query service: {0}")]
    QueryService(#[from] crate::data::query_service::QueryError),

    #[error("Failed to reload ticker registry: {0}")]
    TickerRegistry(#[from] crate::data::ticker_registry::TickerRegistryError),
}

/// Shared data state that can be reloaded.
///
/// Wrapped in Arc<RwLock<>> for concurrent access from multiple handlers.
/// Reload acquires a write lock; reads acquire a read lock.
pub struct DataLoader {
    pub query_service: Arc<RwLock<Arc<QueryService>>>,
    pub ticker_registry: Arc<TickerRegistry>,
    pub data_config: DataConfig,
    pub cache: Arc<CacheStore>,
}

impl DataLoader {
    /// Create a new DataLoader by discovering and loading all data files.
    pub fn new(data_config: &DataConfig, cache: Arc<CacheStore>) -> Result<Self, ReloadError> {
        let files = discover_files(data_config)?;
        let query_service = QueryService::new(files)?;
        let ledger_path = data_config.dir.join("ledger/tucano.db");
        let ticker_registry = TickerRegistry::load(&ledger_path)?;

        Ok(Self {
            query_service: Arc::new(RwLock::new(Arc::new(query_service))),
            ticker_registry: Arc::new(ticker_registry),
            data_config: data_config.clone(),
            cache,
        })
    }

    /// Create a new DataLoader with disabled cache (useful in standalone contexts).
    pub fn new_standalone(data_config: &DataConfig) -> Result<Self, ReloadError> {
        Self::new(data_config, Arc::new(CacheStore::disabled()))
    }

    /// Reload all data files.
    ///
    /// This acquires a write lock on the query service, so concurrent
    /// reads will block until the reload completes.
    pub async fn reload(&self) -> Result<(), ReloadError> {
        tracing::info!("Recarregando dados...");

        // Discover new files
        let files = discover_files(&self.data_config)?;

        // Create new query service
        let new_service = QueryService::new(files)?;

        // Swap the query service
        {
            let mut service = self.query_service.write().await;
            *service = Arc::new(new_service);
        }

        // Reload ticker registry
        let ledger_path = self.data_config.dir.join("ledger/tucano.db");
        self.ticker_registry.reload(&ledger_path)?;

        // Purge cache on reload
        self.cache.purge();

        tracing::info!("Dados recarregados com sucesso");
        Ok(())
    }

    /// Get a read handle to the current query service.
    pub async fn get_query_service(&self) -> Arc<QueryService> {
        let service = self.query_service.read().await;
        Arc::clone(&service)
    }
}
