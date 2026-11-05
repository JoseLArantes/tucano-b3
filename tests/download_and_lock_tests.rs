use std::fs;
use tempfile::tempdir;

use tucano_b3::config::B3Settings;
use tucano_b3::db::locks::InMemoryLockRegistry;
use tucano_b3::services::download_service::B3DownloadService;

#[tokio::test]
async fn test_download_service_rejects_non_b3_host() {
    let settings = B3Settings::default();
    let download_service = B3DownloadService::new(&settings);
    let res = download_service.download("https://example.com/data.xml").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_download_service_allows_local_raw_files() {
    let dir = tempdir().unwrap();
    let raw_dir = dir.path().to_str().unwrap().to_string();

    let mut settings = B3Settings::default();
    settings.b3_raw_data_dir = raw_dir.clone();

    let test_file = dir.path().join("test_file.xml");
    fs::write(&test_file, "<root/>").unwrap();

    let download_service = B3DownloadService::new(&settings);

    // Absolute file path
    let res = download_service.download(test_file.to_str().unwrap()).await.unwrap();
    assert_eq!(res.content, b"<root/>");

    // Relative path
    let res_rel = download_service.download("test_file.xml").await.unwrap();
    assert_eq!(res_rel.content, b"<root/>");

    // Path traversal rejection
    let outside_dir = tempdir().unwrap();
    let outside_file = outside_dir.path().join("outside.xml");
    fs::write(&outside_file, "<root/>").unwrap();

    let res_outside = download_service.download(outside_file.to_str().unwrap()).await;
    assert!(res_outside.is_err());
}

#[test]
fn test_lock_conflict() {
    let registry = InMemoryLockRegistry::new();
    let key = "BVBG_028_INSTRUMENTS_FILE:2026-05-30:2026-05-30";

    let guard1 = registry.acquire(key);
    assert!(guard1.is_ok());

    let guard2 = registry.acquire(key);
    assert!(guard2.is_err());

    drop(guard1);

    let guard3 = registry.acquire(key);
    assert!(guard3.is_ok());
}

