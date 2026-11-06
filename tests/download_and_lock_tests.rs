use std::fs;
use tempfile::tempdir;

use tucano_b3::config::B3Settings;
use tucano_b3::db::locks::InMemoryLockRegistry;
use tucano_b3::services::download_service::B3DownloadService;

#[tokio::test]
async fn test_download_service_rejects_non_b3_host() {
    let settings = B3Settings::default();
    let download_service = B3DownloadService::new(&settings);
    let res = download_service
        .download("https://example.com/data.xml")
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_download_service_allows_local_raw_files() {
    let dir = tempdir().unwrap();
    let raw_dir = dir.path().to_str().unwrap().to_string();

    let settings = B3Settings {
        b3_raw_data_dir: raw_dir.clone(),
        ..Default::default()
    };

    let test_file = dir.path().join("test_file.xml");
    fs::write(&test_file, "<root/>").unwrap();

    let download_service = B3DownloadService::new(&settings);

    // Absolute file path
    let res = download_service
        .download(test_file.to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(res.content, b"<root/>");

    // Relative path
    let res_rel = download_service.download("test_file.xml").await.unwrap();
    assert_eq!(res_rel.content, b"<root/>");

    // Path traversal rejection
    let outside_dir = tempdir().unwrap();
    let outside_file = outside_dir.path().join("outside.xml");
    fs::write(&outside_file, "<root/>").unwrap();

    let res_outside = download_service
        .download(outside_file.to_str().unwrap())
        .await;
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

#[test]
fn test_zip_extraction_depth_limit() {
    use std::io::Write;
    use tucano_b3::db::enums::SourceFileType;
    use tucano_b3::parsers::extract_archive_members;
    use zip::write::{FileOptions, ZipWriter};

    // Create a 6-level deep nested zip (exceeding depth > 4)
    let mut current_bytes = Vec::new();
    {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut current_bytes));
        zw.start_file("innermost.xml", FileOptions::<()>::default())
            .unwrap();
        zw.write_all(b"<root/>").unwrap();
        zw.finish().unwrap();
    }

    for depth in 1..=5 {
        let mut next_bytes = Vec::new();
        {
            let mut zw = ZipWriter::new(std::io::Cursor::new(&mut next_bytes));
            zw.start_file(format!("level_{depth}.zip"), FileOptions::<()>::default())
                .unwrap();
            zw.write_all(&current_bytes).unwrap();
            zw.finish().unwrap();
        }
        current_bytes = next_bytes;
    }

    // Attempt extraction on depth > 4 nested archive
    let result = extract_archive_members(SourceFileType::Bvbg028InstrumentsFile, &current_bytes);
    // At depth > 4, the current implementation silently drops entries, returning Layout error because members is empty
    assert!(result.is_err(), "Deeply nested archive beyond max depth should fail rather than silently produce partial results");
}
