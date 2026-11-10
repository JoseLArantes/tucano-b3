use std::path::Path;
use tempfile::TempDir;
use tucano_b3::middleware::rate_limit::keys::ApiKeyRegistry;

#[test]
fn test_load_missing_file() {
    let registry = ApiKeyRegistry::load(Path::new("/nonexistent/api_keys.toml"));
    assert!(registry.is_ok());
    if let Ok(reg) = registry {
        assert_eq!(reg.count(), 0);
    }
}

#[test]
fn test_load_valid_file() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("api_keys.toml");
    std::fs::write(
        &path,
        r#"
[[keys]]
key = "test-key-123"
name = "Test"
rate_limit = 500
"#,
    )
    .unwrap();

    let registry = ApiKeyRegistry::load(&path).unwrap();
    assert_eq!(registry.count(), 1);
    assert!(registry.exists("test-key-123"));
    assert!(!registry.exists("unknown-key"));
}

#[test]
fn test_validate_key() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("api_keys.toml");
    std::fs::write(
        &path,
        r#"
[[keys]]
key = "valid-key"
name = "Valid"
"#,
    )
    .unwrap();

    let registry = ApiKeyRegistry::load(&path).unwrap();

    let entry = registry.validate("valid-key");
    assert!(entry.is_some());
    if let Some(e) = entry {
        assert_eq!(e.name, "Valid");
    }

    assert!(registry.validate("invalid-key").is_none());
}

#[test]
fn test_rate_limit_custom() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("api_keys.toml");
    std::fs::write(
        &path,
        r#"
[[keys]]
key = "custom-key"
name = "Custom"
rate_limit = 2000
"#,
    )
    .unwrap();

    let registry = ApiKeyRegistry::load(&path).unwrap();
    assert_eq!(registry.rate_limit_for("custom-key", 1000), 2000);
    assert_eq!(registry.rate_limit_for("unknown-key", 1000), 1000);
}

#[test]
fn test_empty_registry() {
    let registry = ApiKeyRegistry::empty();
    assert_eq!(registry.count(), 0);
    assert!(registry.validate("any-key").is_none());
}
