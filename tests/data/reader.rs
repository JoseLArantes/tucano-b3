use std::path::PathBuf;
use tempfile::TempDir;
use tucano_b3::config::DataConfig;
use tucano_b3::data::reader::{discover_files, ReaderError};

#[test]
fn test_discover_files_missing_dir() {
    let config = DataConfig {
        dir: PathBuf::from("/nonexistent/path"),
        reload_interval_seconds: 0,
    };
    let result = discover_files(&config);
    assert!(matches!(result, Err(ReaderError::DataDirNotFound { .. })));
}

#[test]
fn test_discover_files_empty_dir() {
    let temp = TempDir::new().unwrap();
    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };
    let result = discover_files(&config);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert!(files.cotahist_files.is_empty());
    assert!(files.ibov_file.is_none());
}

#[test]
fn test_discover_files_with_cotahist() {
    // This test requires a fixture Parquet file
    // Skip if fixture not available
    let fixture_path = PathBuf::from("tests/fixtures/processed/cotahist/cotahist-2026.parquet");
    if !fixture_path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    let temp = TempDir::new().unwrap();
    let cotahist_dir = temp.path().join("processed/cotahist");
    std::fs::create_dir_all(&cotahist_dir).unwrap();
    std::fs::copy(&fixture_path, cotahist_dir.join("cotahist-2026.parquet")).unwrap();

    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };

    let result = discover_files(&config);
    assert!(result.is_ok());
    let files = result.unwrap();
    assert_eq!(files.cotahist_files.len(), 1);
    assert!(files.cotahist_files[0].row_count > 0);
}
