use tucano_b3::data::query_service::QueryService;
use tucano_b3::data::reader::DiscoveredFiles;

#[test]
fn test_query_service_empty_files() {
    let files = DiscoveredFiles {
        cotahist_files: vec![],
        ibov_file: None,
    };
    let service = QueryService::new(files);
    assert!(service.is_ok());
    let service = service.unwrap();
    assert!(!service.has_table("cotahist"));
    assert!(!service.has_table("ibov"));
}

#[tokio::test]
async fn test_execute_sql_no_table() {
    let files = DiscoveredFiles {
        cotahist_files: vec![],
        ibov_file: None,
    };
    let service = QueryService::new(files).unwrap();
    let result = service.execute_sql("SELECT * FROM cotahist").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_execute_sql_simple() {
    // This test requires fixture data
    // Skip if not available
    let fixture_path =
        std::path::PathBuf::from("tests/fixtures/processed/cotahist/cotahist-2026.parquet");
    if !fixture_path.exists() {
        eprintln!("Skipping: fixture not found");
        return;
    }

    // Build DiscoveredFiles with the fixture
    let files = DiscoveredFiles {
        cotahist_files: vec![tucano_b3::data::reader::ParquetFileInfo {
            path: fixture_path,
            row_count: 100,
            column_count: 21,
            size_bytes: 1024,
        }],
        ibov_file: None,
    };

    let service = QueryService::new(files).unwrap();
    assert!(service.has_table("cotahist"));

    let result = service
        .execute_sql("SELECT ticker, count(*) as cnt FROM cotahist GROUP BY ticker LIMIT 5")
        .await;
    assert!(result.is_ok());
    let batches = result.unwrap();
    assert!(!batches.is_empty());
}
