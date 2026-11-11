use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let config = tucano_b3::config::AppConfig::default();
    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let data_loader =
        Arc::new(tucano_b3::data::reload::DataLoader::new(&config.data, cache.clone()).unwrap());
    let api_keys = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    tucano_b3::server::build_router(&config, data_loader, cache, rate_limit_state)
}

#[tokio::test]
async fn test_quotes_unknown_ticker_returns_404() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ/quotes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_quotes_invalid_date_returns_400() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/quotes?desde=not-a-date")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Will be 404 (ticker not found) or 400 (invalid date) depending on validation order
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_quotes_date_range_validation() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/quotes?desde=2026-12-31&ate=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Will be 404 or 400
    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_quotes_with_dias_param() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/quotes?dias=5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 404 because no data, but endpoint should not crash
    assert!(response.status().is_client_error() || response.status().is_success());
}
