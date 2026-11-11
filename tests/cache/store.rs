use std::time::Duration;
use tucano_b3::cache::store::CacheStore;
use tucano_b3::config::CacheConfig;

fn test_config() -> CacheConfig {
    CacheConfig {
        enabled: true,
        ttl_seconds: 300,
        max_entries: 5,
    }
}

#[test]
fn test_insert_and_get() {
    let store = CacheStore::new(&test_config());
    store.insert(
        "key1".to_string(),
        b"data1".to_vec(),
        200,
        "application/json".to_string(),
    );

    let entry = store.get("key1");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().body, b"data1");
}

#[test]
fn test_get_missing_key() {
    let store = CacheStore::new(&test_config());
    let entry = store.get("nonexistent");
    assert!(entry.is_none());
}

#[test]
fn test_disabled_cache() {
    let store = CacheStore::disabled();
    store.insert(
        "key1".to_string(),
        b"data".to_vec(),
        200,
        "application/json".to_string(),
    );
    assert!(store.get("key1").is_none());
    assert_eq!(store.len(), 0);
}

#[test]
fn test_expired_entry() {
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 0, // Immediate expiry
        max_entries: 10,
    };
    let store = CacheStore::new(&config);
    store.insert(
        "key1".to_string(),
        b"data".to_vec(),
        200,
        "application/json".to_string(),
    );

    // Entry should be expired immediately
    std::thread::sleep(Duration::from_millis(10));
    assert!(store.get("key1").is_none());
}

#[test]
fn test_lru_eviction() {
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 300,
        max_entries: 3,
    };
    let store = CacheStore::new(&config);

    // Insert 3 entries (at capacity)
    store.insert(
        "key1".to_string(),
        b"data1".to_vec(),
        200,
        "application/json".to_string(),
    );
    store.insert(
        "key2".to_string(),
        b"data2".to_vec(),
        200,
        "application/json".to_string(),
    );
    store.insert(
        "key3".to_string(),
        b"data3".to_vec(),
        200,
        "application/json".to_string(),
    );

    assert_eq!(store.len(), 3);

    // Insert 4th entry — should evict oldest
    store.insert(
        "key4".to_string(),
        b"data4".to_vec(),
        200,
        "application/json".to_string(),
    );

    assert_eq!(store.len(), 3);
    assert_eq!(store.eviction_count(), 1);
}

#[test]
fn test_purge() {
    let store = CacheStore::new(&test_config());
    store.insert(
        "key1".to_string(),
        b"data1".to_vec(),
        200,
        "application/json".to_string(),
    );
    store.insert(
        "key2".to_string(),
        b"data2".to_vec(),
        200,
        "application/json".to_string(),
    );

    assert_eq!(store.len(), 2);

    store.purge();

    assert_eq!(store.len(), 0);
    assert!(store.get("key1").is_none());
}

#[test]
fn test_hit_miss_counts() {
    let store = CacheStore::new(&test_config());
    store.insert(
        "key1".to_string(),
        b"data".to_vec(),
        200,
        "application/json".to_string(),
    );

    // Hit
    store.get("key1");
    assert_eq!(store.hit_count(), 1);

    // Miss
    store.get("key2");
    assert_eq!(store.miss_count(), 1);
}

#[test]
fn test_hit_rate() {
    let store = CacheStore::new(&test_config());
    store.insert(
        "key1".to_string(),
        b"data".to_vec(),
        200,
        "application/json".to_string(),
    );

    store.get("key1"); // hit
    store.get("key2"); // miss

    assert_eq!(store.hit_rate(), 50.0);
}

#[test]
fn test_cleanup_expired() {
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 0,
        max_entries: 10,
    };
    let store = CacheStore::new(&config);
    store.insert(
        "key1".to_string(),
        b"data".to_vec(),
        200,
        "application/json".to_string(),
    );

    std::thread::sleep(Duration::from_millis(10));
    store.cleanup_expired();

    assert_eq!(store.len(), 0);
}

#[test]
fn test_max_body_size_not_cached() {
    let store = CacheStore::new(&test_config());
    let large_body = vec![0u8; tucano_b3::cache::store::MAX_CACHE_BODY_SIZE + 1];
    store.insert(
        "large_key".to_string(),
        large_body,
        200,
        "application/json".to_string(),
    );

    assert!(store.get("large_key").is_none());
    assert_eq!(store.len(), 0);
}
