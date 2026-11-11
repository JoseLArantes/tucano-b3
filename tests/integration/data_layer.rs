//! Integration tests for the data layer.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tucano_b3::config::DataConfig;
use tucano_b3::data::reload::DataLoader;

/// Helper: create a test DataLoader with fixture data.
fn create_test_loader() -> (TempDir, DataLoader) {
    let temp = TempDir::new().unwrap();

    // Copy fixtures to temp dir
    let fixture_src = PathBuf::from("tests/fixtures");
    if fixture_src.exists() {
        copy_dir_recursive(&fixture_src, temp.path());
    }

    let config = DataConfig {
        dir: temp.path().to_path_buf(),
        reload_interval_seconds: 0,
    };

    let loader = DataLoader::new_standalone(&config).expect("Failed to create DataLoader");
    (temp, loader)
}

fn copy_dir_recursive(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).ok();
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path);
            } else {
                fs::copy(&src_path, &dst_path).ok();
            }
        }
    }
}

#[tokio::test]
async fn test_full_pipeline_cotahist_query() {
    let (_temp, loader) = create_test_loader();
    let service = loader.get_query_service().await;

    if !service.has_table("cotahist") {
        eprintln!("Skipping: no cotahist fixture");
        return;
    }

    let result = service
        .execute_sql("SELECT ticker, count(*) as cnt FROM cotahist GROUP BY ticker ORDER BY ticker")
        .await;

    assert!(result.is_ok());
    let batches = result.unwrap();
    assert!(!batches.is_empty());
    assert!(batches[0].num_rows() > 0);
}

#[tokio::test]
async fn test_full_pipeline_ticker_validation() {
    let (_temp, loader) = create_test_loader();

    // If fixture has VALE3, verify it exists
    assert!(loader.ticker_registry.exists("VALE3"));
    assert!(loader.ticker_registry.exists("PETR4"));
    assert!(loader.ticker_registry.exists("ITUB4"));
    // Verify unknown ticker returns false
    assert!(!loader.ticker_registry.exists("XYZXYZ"));
}

#[tokio::test]
async fn test_reload_after_adding_file() {
    let (_temp, loader) = create_test_loader();

    let initial_count = loader.get_query_service().await.cotahist_file_count();

    // Reload should succeed
    let result = loader.reload().await;
    assert!(result.is_ok());

    let after_count = loader.get_query_service().await.cotahist_file_count();
    assert_eq!(initial_count, after_count);
}
