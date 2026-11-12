//! Integration tests for OpenAPI and Swagger UI.

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
    let rate_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    let metrics = Arc::new(tucano_b3::metrics::AppMetrics::new());
    let start_time = std::time::Instant::now();
    tucano_b3::server::build_router(&config, data_loader, cache, rate_state, metrics, start_time)
}

#[tokio::test]
async fn test_openapi_json_returns_200() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_openapi_json_is_valid() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify structure
    assert_eq!(json["openapi"], "3.0.3");
    assert_eq!(json["info"]["title"], "tucano-b3");
    assert!(json["paths"].is_object());
    assert!(json["components"]["schemas"].is_object());
}

#[tokio::test]
async fn test_openapi_has_all_endpoints() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let paths = json["paths"].as_object().unwrap();

    // COTAHIST endpoints
    assert!(paths.contains_key("/cotahist/tickers"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/quotes"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/latest"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/series"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/pre-ult/min"));
    assert!(paths.contains_key("/cotahist/tickers/{ticker}/pre-ult/max"));
    assert!(paths.contains_key("/cotahist/dates/{date}/quotes"));

    // IBOV endpoints
    assert!(paths.contains_key("/ibov/history"));
    assert!(paths.contains_key("/ibov/latest"));

    // System endpoints
    assert!(paths.contains_key("/health"));
    assert!(paths.contains_key("/info"));
}

#[tokio::test]
async fn test_openapi_has_security_scheme() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let security = &json["components"]["securitySchemes"]["ApiKeyAuth"];
    assert_eq!(security["type"], "apiKey");
    assert_eq!(security["name"], "X-API-Key");
    assert_eq!(security["in"], "header");
}

#[tokio::test]
async fn test_openapi_has_error_schema() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["components"]["schemas"]["ErrorResponse"].is_object());
    assert!(json["components"]["schemas"]["ErrorDetail"].is_object());
}

#[tokio::test]
async fn test_swagger_ui_returns_200() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_openapi_descriptions_in_portuguese() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Check that info description is in Portuguese
    let description = json["info"]["description"].as_str().unwrap();
    assert!(description.contains("API REST"));
    assert!(description.contains("B3"));
}
