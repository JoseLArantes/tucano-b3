//! Cache metrics for Prometheus exposition.

use crate::cache::store::CacheStore;
use serde::Serialize;
use utoipa::ToSchema;

/// Cache statistics for the /api/v1/info endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct CacheStats {
    /// Whether cache is enabled
    pub enabled: bool,
    /// Number of cached entries
    pub entries: usize,
    /// Maximum allowed entries
    pub max_entries: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache evictions
    pub evictions: u64,
    /// Cache hit rate percentage (0.0 to 100.0)
    pub hit_rate_pct: f64,
}

impl CacheStats {
    /// Collect current cache statistics from store.
    pub fn from_store(store: &CacheStore) -> Self {
        Self {
            enabled: store.is_enabled(),
            entries: store.len(),
            max_entries: 0,
            hits: store.hit_count(),
            misses: store.miss_count(),
            evictions: store.eviction_count(),
            hit_rate_pct: store.hit_rate(),
        }
    }

    /// Collect current cache statistics with maximum entries from config.
    pub fn from_store_with_max(store: &CacheStore, max_entries: usize) -> Self {
        Self {
            enabled: store.is_enabled(),
            entries: store.len(),
            max_entries,
            hits: store.hit_count(),
            misses: store.miss_count(),
            evictions: store.eviction_count(),
            hit_rate_pct: store.hit_rate(),
        }
    }
}
