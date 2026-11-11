use std::sync::Arc;
use tempfile::TempDir;
use tucano_b3::cache::store::CacheStore;
use tucano_b3::config::DataConfig;
use tucano_b3::data::reload::DataLoader;

#[test]
fn test_data_loader_empty_dir() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let cache = Arc::new(CacheStore::disabled());
    let loader = DataLoader::new(&config, cache);
    assert!(loader.is_ok());
}

#[tokio::test]
async fn test_data_loader_reload_empty() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let cache = Arc::new(CacheStore::disabled());
    let loader = DataLoader::new(&config, cache).unwrap();
    let result = loader.reload().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_data_loader_reload_purges_cache() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let cache_config = tucano_b3::config::CacheConfig {
        enabled: true,
        ttl_seconds: 300,
        max_entries: 10,
    };
    let cache = Arc::new(CacheStore::new(&cache_config));
    cache.insert(
        "test_key".to_string(),
        b"data".to_vec(),
        200,
        "application/json".to_string(),
    );
    assert_eq!(cache.len(), 1);

    let loader = DataLoader::new(&config, cache.clone()).unwrap();
    let result = loader.reload().await;
    assert!(result.is_ok());
    assert_eq!(cache.len(), 0);
}

#[tokio::test]
async fn test_get_query_service() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let cache = Arc::new(CacheStore::disabled());
    let loader = DataLoader::new(&config, cache).unwrap();
    let service = loader.get_query_service().await;
    assert!(!service.has_table("cotahist"));
}
