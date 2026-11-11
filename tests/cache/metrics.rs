use tucano_b3::cache::metrics::CacheStats;
use tucano_b3::cache::store::CacheStore;
use tucano_b3::config::CacheConfig;

#[test]
fn test_cache_stats_from_store() {
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 300,
        max_entries: 50,
    };
    let store = CacheStore::new(&config);
    store.insert(
        "k1".to_string(),
        b"v1".to_vec(),
        200,
        "application/json".to_string(),
    );
    let _ = store.get("k1"); // hit
    let _ = store.get("k2"); // miss

    let stats = CacheStats::from_store(&store);
    assert!(stats.enabled);
    assert_eq!(stats.entries, 1);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.hit_rate_pct, 50.0);

    let json = serde_json::to_string(&stats).unwrap();
    assert!(json.contains("\"enabled\":true"));
    assert!(json.contains("\"entries\":1"));
}
