//! Integration tests for IBOV endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn test_app_with_fixtures() -> Option<axum::Router> {
    let fixture_dir = std::path::PathBuf::from("tests/fixtures");
    if !fixture_dir
        .join("processed/ibov/ibov-historical.parquet")
        .exists()
    {
        return None;
    }

    let config = tucano_b3::config::AppConfig {
        data: tucano_b3::config::DataConfig {
            dir: fixture_dir,
            reload_interval_seconds: 0,
        },
        ..Default::default()
    };

    let data_loader =
        std::sync::Arc::new(tucano_b3::data::reload::DataLoader::new(&config.data).ok()?);

    Some(tucano_b3::server::build_router(&config, data_loader))
}

#[tokio::test]
async fn test_ibov_history_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no IBOV fixture");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["codigo"], "IBOV");
    assert!(json["data"]["history"].is_array());
    assert!(json["data"]["paginacao"]["total"].is_number());
    assert!(json["meta"]["request_id"].is_string());
}

#[tokio::test]
async fn test_ibov_history_with_date_filter() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no IBOV fixture");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?dias=5&ordem=desc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let history = json["data"]["history"].as_array().unwrap();
    assert!(history.len() <= 5);
}

#[tokio::test]
async fn test_ibov_latest_with_data() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no IBOV fixture");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["codigo"], "IBOV");
    assert!(json["data"]["data"].is_string());
    assert!(json["data"]["pre_ult"].is_number());
    assert!(json["data"]["var_pct"].is_number());
}

#[tokio::test]
async fn test_ibov_history_envelope() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no IBOV fixture");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?por_pagina=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify envelope
    assert!(json["data"].is_object());
    assert!(json["meta"].is_object());
    assert!(json["meta"]["cached"].is_boolean());
    assert!(json["meta"]["query_time_ms"].is_number());
}

#[tokio::test]
async fn test_ibov_history_invalid_date_filter() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no IBOV fixture");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?desde=not-a-date")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_ibov_history_reversed_dates() {
    let app = match test_app_with_fixtures() {
        Some(app) => app,
        None => {
            eprintln!("Skipping: no IBOV fixture");
            return;
        }
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?desde=2026-12-31&ate=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
