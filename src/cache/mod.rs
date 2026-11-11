//! In-memory cache for tucano-b3.
//!
//! Uses DashMap for concurrent access with lock-free reads.
//! Supports TTL-based expiration and LRU eviction.

pub mod key;
pub mod metrics;
pub mod store;

pub use key::CacheKey;
pub use metrics::CacheStats;
pub use store::CacheStore;
