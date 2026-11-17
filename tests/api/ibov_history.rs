use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn test_ibov_history_no_data() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/metrics/ibov/history")
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
                .uri("/api/v1/metrics/ibov/history?desde=not-a-date")
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
                .uri("/api/v1/metrics/ibov/history?dias=5")
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
                .uri("/api/v1/metrics/ibov/history?pagina=1&por_pagina=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 404 because no data, but should not crash
    assert!(response.status().is_client_error() || response.status().is_success());
}
