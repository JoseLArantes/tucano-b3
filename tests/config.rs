use std::path::Path;
use std::sync::Mutex;
use tucano_b3::config::AppConfig;

static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_default_config() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let config = AppConfig::default();
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.workers, 0);
    assert_eq!(config.data.dir, Path::new("./data"));
    assert!(config.cache.enabled);
    assert_eq!(config.cache.ttl_seconds, 300);
    assert_eq!(config.cache.max_entries, 10_000);
    assert!(config.rate_limit.enabled);
    assert_eq!(config.logging.level, "info");
    assert!(config.metrics.enabled);
}

#[test]
fn test_load_from_file() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let config = AppConfig::load(Some(Path::new("config.toml")));
    assert!(config.is_ok());
    let config = config.unwrap();
    assert_eq!(config.server.port, 8080);
}

#[test]
fn test_load_missing_file() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let config = AppConfig::load(Some(Path::new("nonexistent.toml")));
    assert!(config.is_err());
}

#[test]
fn test_load_no_file_uses_defaults() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let config = AppConfig::load(None);
    assert!(config.is_ok());
    let config = config.unwrap();
    assert_eq!(config.server.port, 8080);
}

#[test]
fn test_env_override_port() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("TUCANO_SERVER_PORT", "9090");
    let config = AppConfig::load(None).unwrap();
    assert_eq!(config.server.port, 9090);
    std::env::remove_var("TUCANO_SERVER_PORT");
}

#[test]
fn test_env_override_data_dir() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("TUCANO_DATA_DIR", "/mnt/data");
    let config = AppConfig::load(None).unwrap();
    assert_eq!(config.data.dir, Path::new("/mnt/data"));
    std::env::remove_var("TUCANO_DATA_DIR");
}

#[test]
fn test_env_override_log_level() {
    let _lock = ENV_MUTEX.lock().unwrap();
    std::env::set_var("TUCANO_LOG_LEVEL", "debug");
    let config = AppConfig::load(None).unwrap();
    assert_eq!(config.logging.level, "debug");
    std::env::remove_var("TUCANO_LOG_LEVEL");
}

#[test]
fn test_validation_zero_port() {
    let mut config = AppConfig::default();
    config.server.port = 0;
    // Validation happens in load(), so we test the validate method directly
    assert!(config.validate().is_err());
}

#[test]
fn test_validation_zero_ttl_with_cache_enabled() {
    let mut config = AppConfig::default();
    config.cache.ttl_seconds = 0;
    config.cache.enabled = true;
    assert!(config.validate().is_err());
}
