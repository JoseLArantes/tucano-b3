//! Cache key generation.
//!
//! Generates deterministic cache keys from HTTP request attributes.
//! Key = BLAKE3(method + path + sorted query params)

use blake3::Hasher;

/// Generate a cache key from HTTP request attributes.
///
/// The key is a BLAKE3 hash of:
/// - HTTP method (always GET for this API)
/// - Path (e.g., "/api/v1/cotahist/tickers/VALE3/quotes")
/// - Sorted query parameters (e.g., "dias=5&ordem=desc")
///
/// Sorting query params ensures "a=1&b=2" and "b=2&a=1" produce the same key.
pub fn generate_cache_key(method: &str, path: &str, query: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(method.as_bytes());
    hasher.update(b"|");
    hasher.update(path.as_bytes());
    hasher.update(b"|");
    hasher.update(sort_query_params(query).as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Sort query parameters alphabetically for deterministic key generation.
///
/// "b=2&a=1" → "a=1&b=2"
pub fn sort_query_params(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }

    let mut params: Vec<&str> = query.split('&').collect();
    params.sort();
    params.join("&")
}

/// Determine if a path should be cached.
///
/// Health, metrics, info, and documentation endpoints are never cached.
pub fn is_cacheable(path: &str) -> bool {
    !path.starts_with("/api/v1/health")
        && !path.starts_with("/api/v1/metrics")
        && !path.starts_with("/api/v1/info")
        && !path.starts_with("/metrics")
        && !path.starts_with("/api/docs")
        && !path.starts_with("/health")
        && !path.starts_with("/redoc")
        && !path.starts_with("/api/redoc")
}

/// CacheKey type alias for key representation.
pub type CacheKey = String;
