use tempfile::TempDir;
use tucano_b3::config::DataConfig;
use tucano_b3::data::reload::DataLoader;

#[test]
fn test_data_loader_empty_dir() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let loader = DataLoader::new(&config);
    assert!(loader.is_ok());
}

#[tokio::test]
async fn test_data_loader_reload_empty() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let loader = DataLoader::new(&config).unwrap();
    let result = loader.reload().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_query_service() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let loader = DataLoader::new(&config).unwrap();
    let service = loader.get_query_service().await;
    assert!(!service.has_table("cotahist"));
}
