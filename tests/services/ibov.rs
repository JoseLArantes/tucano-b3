use std::path::PathBuf;
use tucano_b3::config::DataConfig;
use tucano_b3::data::query_service::QueryService;
use tucano_b3::data::reader::discover_files;
use tucano_b3::models::pagination::PaginationParams;
use tucano_b3::services::ibov::IbovService;

#[test]
fn test_ibov_validate_date_valid() {
    assert!(IbovService::validate_date("2026-11-10").is_ok());
}

#[test]
fn test_ibov_validate_date_invalid() {
    assert!(IbovService::validate_date("invalid").is_err());
}

#[test]
fn test_ibov_validate_date_wrong_format() {
    assert!(IbovService::validate_date("10/11/2026").is_err());
}

#[tokio::test]
async fn test_ibov_service_operations() {
    let fixture_dir = PathBuf::from("tests/fixtures");
    if !fixture_dir.exists() {
        return;
    }

    let config = DataConfig {
        dir: fixture_dir.clone(),
        reload_interval_seconds: 0,
    };

    let discovered = discover_files(&config).expect("failed to discover");
    let query_service = QueryService::new(discovered).expect("failed to build query service");

    let pagination = PaginationParams {
        pagina: 1,
        por_pagina: 10,
    };

    // 1. Get history
    let (entries, total) =
        IbovService::get_history(&query_service, None, None, None, "desc", &pagination)
            .await
            .expect("get_history failed");

    assert_eq!(total, 2);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].codigo, "IBOV");

    // 2. Get latest
    let latest = IbovService::get_latest(&query_service)
        .await
        .expect("get_latest failed");

    assert_eq!(latest.codigo, "IBOV");
    assert!(latest.pre_ult.is_some());
    assert!(latest.var_pct.is_some());

    // 3. Invalid date
    let err =
        IbovService::get_history(&query_service, Some("bad"), None, None, "desc", &pagination)
            .await;
    assert!(err.is_err());

    // 4. Reversed date
    let err = IbovService::get_history(
        &query_service,
        Some("2026-12-31"),
        Some("2026-01-01"),
        None,
        "desc",
        &pagination,
    )
    .await;
    assert!(err.is_err());

    // 5. Filter with dias
    let (entries_dias, _) =
        IbovService::get_history(&query_service, None, None, Some(1), "desc", &pagination)
            .await
            .expect("get_history with dias failed");
    assert_eq!(entries_dias.len(), 1);
}
