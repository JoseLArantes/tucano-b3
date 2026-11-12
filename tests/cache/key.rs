use tucano_b3::cache::key::{generate_cache_key, is_cacheable, sort_query_params};

#[test]
fn test_generate_cache_key_deterministic() {
    let key1 = generate_cache_key("GET", "/api/v1/cotahist/tickers/VALE3", "");
    let key2 = generate_cache_key("GET", "/api/v1/cotahist/tickers/VALE3", "");
    assert_eq!(key1, key2);
}

#[test]
fn test_generate_cache_key_different_paths() {
    let key1 = generate_cache_key("GET", "/api/v1/cotahist/tickers/VALE3", "");
    let key2 = generate_cache_key("GET", "/api/v1/cotahist/tickers/PETR4", "");
    assert_ne!(key1, key2);
}

#[test]
fn test_generate_cache_key_different_queries() {
    let key1 = generate_cache_key("GET", "/api/v1/cotahist/tickers", "pagina=1");
    let key2 = generate_cache_key("GET", "/api/v1/cotahist/tickers", "pagina=2");
    assert_ne!(key1, key2);
}

#[test]
fn test_sort_query_params() {
    assert_eq!(sort_query_params("b=2&a=1"), "a=1&b=2");
    assert_eq!(sort_query_params("a=1"), "a=1");
    assert_eq!(sort_query_params(""), "");
}

#[test]
fn test_sorted_params_same_key() {
    let key1 = generate_cache_key("GET", "/path", "a=1&b=2");
    let key2 = generate_cache_key("GET", "/path", "b=2&a=1");
    assert_eq!(key1, key2);
}

#[test]
fn test_is_cacheable() {
    assert!(is_cacheable("/api/v1/cotahist/tickers"));
    assert!(is_cacheable("/api/v1/ibov/history"));
    assert!(!is_cacheable("/api/v1/health"));
    assert!(!is_cacheable("/api/v1/metrics"));
    assert!(!is_cacheable("/api/v1/info"));
    assert!(!is_cacheable("/metrics"));
    assert!(!is_cacheable("/api/docs"));
    assert!(!is_cacheable("/api/docs/openapi.json"));
}
