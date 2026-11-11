use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;
use tucano_b3::config::RateLimitConfig;
use tucano_b3::middleware::rate_limit::keys::ApiKeyRegistry;
use tucano_b3::middleware::rate_limit::layer::{RateLimitLayer, RateLimitState, API_KEY_HEADER};

fn test_state_with_keys() -> Arc<RateLimitState> {
    let temp = TempDir::new().unwrap();
    let keys_path = temp.path().join("api_keys.toml");
    std::fs::write(
        &keys_path,
        r#"
[[keys]]
key = "valid-test-key"
name = "Test"
rate_limit = 5
"#,
    )
    .unwrap();

    let config = RateLimitConfig {
        enabled: true,
        default_requests_per_minute: 2,
        api_key_requests_per_minute: 5,
    };

    let keys = Arc::new(ApiKeyRegistry::load(&keys_path).unwrap());
    Arc::new(RateLimitState::new(keys, config))
}

fn test_app(state: Arc<RateLimitState>) -> Router {
    let metrics = Arc::new(tucano_b3::metrics::AppMetrics::new());
    Router::new()
        .route("/api/v1/test", get(|| async { "ok" }))
        .layer(RateLimitLayer::new(state, metrics))
}

#[tokio::test]
async fn test_valid_api_key_gets_higher_limit() {
    let state = test_state_with_keys();
    let app = test_app(state);

    // With valid API key, should get 5 requests
    for i in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/test")
                    .header(API_KEY_HEADER, "valid-test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200, "Request {} should succeed", i + 1);
    }

    // 6th should fail
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .header(API_KEY_HEADER, "valid-test-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 429);
}

#[tokio::test]
async fn test_invalid_api_key_falls_back_to_ip() {
    let state = test_state_with_keys();
    let app = test_app(state);

    // With invalid API key, should get default limit (2)
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/test")
                    .header(API_KEY_HEADER, "invalid-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
    }

    // 3rd should fail (default limit is 2)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .header(API_KEY_HEADER, "invalid-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 429);
}

#[tokio::test]
async fn test_no_api_key_uses_ip() {
    let state = test_state_with_keys();
    let app = test_app(state);

    // Without API key, should get default limit (2)
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
}
