//! Cache store implementation.
//!
//! Thread-safe, TTL-based, LRU-evicting cache.
//! Uses DashMap for O(1) concurrent reads.

use crate::config::CacheConfig;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A single cache entry with metadata.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached response body (serialized JSON bytes)
    pub body: Vec<u8>,
    /// HTTP status code
    pub status: u16,
    /// Content-Type header value
    pub content_type: String,
    /// When this entry was created
    pub created_at: Instant,
    /// When this entry was last accessed (for LRU)
    pub last_accessed: Instant,
    /// Time-to-live for this entry
    pub ttl: Duration,
}

impl CacheEntry {
    /// Check if this entry has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Thread-safe cache store with TTL and LRU eviction.
pub struct CacheStore {
    entries: DashMap<String, CacheEntry>,
    max_entries: usize,
    ttl: Duration,
    enabled: bool,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl CacheStore {
    /// Create a new CacheStore from configuration.
    pub fn new(config: &CacheConfig) -> Self {
        Self {
            entries: DashMap::new(),
            max_entries: config.max_entries,
            ttl: Duration::from_secs(config.ttl_seconds),
            enabled: config.enabled,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Create a disabled cache (no-op).
    pub fn disabled() -> Self {
        Self {
            entries: DashMap::new(),
            max_entries: 0,
            ttl: Duration::from_secs(0),
            enabled: false,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get a cached entry by key.
    ///
    /// Returns None if:
    /// - Cache is disabled
    /// - Key not found
    /// - Entry has expired
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        if !self.enabled {
            return None;
        }

        match self.entries.get(key) {
            Some(entry) => {
                if entry.is_expired() {
                    drop(entry);
                    self.entries.remove(key);
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    None
                } else {
                    drop(entry);
                    if let Some(mut entry) = self.entries.get_mut(key) {
                        entry.last_accessed = Instant::now();
                        self.hits.fetch_add(1, Ordering::Relaxed);
                        Some(entry.clone())
                    } else {
                        self.misses.fetch_add(1, Ordering::Relaxed);
                        None
                    }
                }
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Insert a new entry into the cache.
    ///
    /// If the cache is at capacity, evicts the least recently used entry.
    pub fn insert(&self, key: String, body: Vec<u8>, status: u16, content_type: String) {
        if !self.enabled {
            return;
        }

        // Evict if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_lru();
        }

        let entry = CacheEntry {
            body,
            status,
            content_type,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            ttl: self.ttl,
        };

        self.entries.insert(key, entry);
    }

    /// Evict the least recently used entry.
    fn evict_lru(&self) {
        let mut oldest: Option<(String, Instant)> = None;

        for entry in self.entries.iter() {
            let key = entry.key().clone();
            let accessed = entry.value().last_accessed;
            match &oldest {
                None => oldest = Some((key, accessed)),
                Some((_, oldest_time)) if accessed < *oldest_time => {
                    oldest = Some((key, accessed));
                }
                _ => {}
            }
        }

        if let Some((key, _)) = oldest {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(key = %key, "Cache: entrada evitada (LRU)");
        }
    }

    /// Purge all entries from the cache.
    ///
    /// Called on data reload to ensure stale data is not served.
    pub fn purge(&self) {
        let count = self.entries.len();
        self.entries.clear();
        tracing::info!(entries = count, "Cache: todas as entradas removidas");
    }

    /// Get current number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Check if cache is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get cache hit count.
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get cache miss count.
    pub fn miss_count(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Get cache eviction count.
    pub fn eviction_count(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Get cache hit rate as a percentage (0.0 to 100.0).
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits.load(Ordering::Relaxed) + self.misses.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        (self.hits.load(Ordering::Relaxed) as f64 / total as f64) * 100.0
    }

    /// Remove expired entries.
    ///
    /// Called periodically or on demand to free memory.
    pub fn cleanup_expired(&self) {
        let expired_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.value().is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            self.entries.remove(&key);
        }
    }
}
