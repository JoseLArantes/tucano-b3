use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let config = tucano_b3::config::AppConfig::default();
    let data_loader = Arc::new(tucano_b3::data::reload::DataLoader::new(&config.data).unwrap());
    let api_keys = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    tucano_b3::server::build_router(&config, data_loader, rate_limit_state)
}

#[tokio::test]
async fn test_ibov_history_no_data() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 404 because no IBOV data loaded
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_ibov_history_invalid_date() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?desde=not-a-date")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 400 or 404 (invalid date or no data)
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_ibov_history_with_dias() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?dias=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 404 because no data, but should not crash
    assert!(response.status().is_client_error() || response.status().is_success());
}

#[tokio::test]
async fn test_ibov_history_pagination() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/ibov/history?pagina=1&por_pagina=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 404 because no data, but should not crash
    assert!(response.status().is_client_error() || response.status().is_success());
}
