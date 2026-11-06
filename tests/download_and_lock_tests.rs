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

#[tokio::test]
async fn test_transactional_advisory_lock_fencing() {
    use tucano_b3::db::create_pool;
    use tucano_b3::db::locks::try_acquire_pg_advisory_xact_lock_tx;

    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string());
    let pool = create_pool(&url)
        .await
        .expect("Integration test requires PostgreSQL; connect failed");

    let lock_key = "test:concurrency:fencing:xact:2026-11-06";

    // Transaction 1 acquires the transactional lock
    let mut tx1 = pool.begin().await.expect("Failed to begin tx1");
    let acq1 = try_acquire_pg_advisory_xact_lock_tx(&mut tx1, lock_key)
        .await
        .expect("Failed to execute lock query on tx1");
    assert!(acq1, "First transaction must acquire the lock");

    // Transaction 2 attempts to acquire the exact same lock while tx1 is still in flight
    let mut tx2 = pool.begin().await.expect("Failed to begin tx2");
    let acq2 = try_acquire_pg_advisory_xact_lock_tx(&mut tx2, lock_key)
        .await
        .expect("Failed to execute lock query on tx2");
    assert!(
        !acq2,
        "Second concurrent transaction must be rejected (fenced) while tx1 holds it"
    );

    // Rollback tx1 (releasing the transactional lock)
    tx1.rollback().await.expect("Failed to rollback tx1");

    // Now Transaction 2 (or a new transaction) can acquire it successfully
    let mut tx3 = pool.begin().await.expect("Failed to begin tx3");
    let acq3 = try_acquire_pg_advisory_xact_lock_tx(&mut tx3, lock_key)
        .await
        .expect("Failed to execute lock query on tx3");
    assert!(
        acq3,
        "Subsequent transaction must acquire the lock after previous tx released it"
    );
    tx3.rollback().await.expect("Failed to rollback tx3");
}

#[tokio::test]
async fn test_session_advisory_lock_lifecycle() {
    use sqlx::Connection;
    use tucano_b3::db::locks::{release_session_advisory_lock, try_acquire_session_advisory_lock};

    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string());
    let mut conn1 = sqlx::PgConnection::connect(&url)
        .await
        .expect("Failed to connect conn1");
    let mut conn2 = sqlx::PgConnection::connect(&url)
        .await
        .expect("Failed to connect conn2");

    let lock_key = "test:session:lock:lifecycle:2026-11-06";

    // Connection 1 acquires session lock
    let acq1 = try_acquire_session_advisory_lock(&mut conn1, lock_key)
        .await
        .expect("Failed lock query on conn1");
    assert!(acq1, "Connection 1 must acquire session lock");

    // Connection 2 fails to acquire it
    let acq2 = try_acquire_session_advisory_lock(&mut conn2, lock_key)
        .await
        .expect("Failed lock query on conn2");
    assert!(
        !acq2,
        "Connection 2 must be fenced while conn1 holds session lock"
    );

    // Connection 1 explicitly releases the session lock
    let rel = release_session_advisory_lock(&mut conn1, lock_key)
        .await
        .expect("Failed unlock on conn1");
    assert!(rel, "Connection 1 must successfully release lock");

    // Connection 2 can now acquire it
    let acq3 = try_acquire_session_advisory_lock(&mut conn2, lock_key)
        .await
        .expect("Failed lock query on conn2");
    assert!(acq3, "Connection 2 must acquire lock after release");
    let _ = release_session_advisory_lock(&mut conn2, lock_key).await;
}
