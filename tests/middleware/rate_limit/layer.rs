use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::config::RateLimitConfig;
use tucano_b3::middleware::rate_limit::keys::ApiKeyRegistry;
use tucano_b3::middleware::rate_limit::layer::{RateLimitLayer, RateLimitState};

fn test_state(limit: u32) -> Arc<RateLimitState> {
    let config = RateLimitConfig {
        enabled: true,
        default_requests_per_minute: limit,
        api_key_requests_per_minute: limit * 10,
    };
    let keys = Arc::new(ApiKeyRegistry::empty());
    Arc::new(RateLimitState::new(keys, config))
}

fn test_app(state: Arc<RateLimitState>) -> Router {
    let metrics = Arc::new(tucano_b3::metrics::AppMetrics::new());
    Router::new()
        .route("/api/v1/test", get(|| async { "ok" }))
        .route("/api/v1/health", get(|| async { "healthy" }))
        .layer(RateLimitLayer::new(state, metrics))
}

#[tokio::test]
async fn test_rate_limit_allows_within_limit() {
    let state = test_state(10);
    let app = test_app(state);

    for _ in 0..5 {
        let response = app
            .clone()
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
}

#[tokio::test]
async fn test_rate_limit_blocks_over_limit() {
    let state = test_state(3);
    let app = test_app(state);

    // Consume all 3 tokens
    for _ in 0..3 {
        let response = app
            .clone()
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

    // 4th request should be rate limited
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 429);
    assert!(response.headers().contains_key("retry-after"));
}

#[tokio::test]
async fn test_health_never_rate_limited() {
    let state = test_state(1); // Very low limit
    let app = test_app(state);

    // Make many health requests — should never be limited
    for _ in 0..10 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }
}

#[tokio::test]
async fn test_rate_limit_headers_present() {
    let state = test_state(100);
    let app = test_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.headers().contains_key("x-ratelimit-remaining"));
    assert!(response.headers().contains_key("x-ratelimit-limit"));
}

#[tokio::test]
async fn test_rate_limit_429_body() {
    let state = test_state(1);
    let app = test_app(state);

    // Consume the one token
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Next request gets 429
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 429);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "RATE_LIMITED");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Limite"));
}

#[tokio::test]
async fn test_disabled_rate_limit() {
    let config = RateLimitConfig {
        enabled: false,
        default_requests_per_minute: 1,
        api_key_requests_per_minute: 1,
    };
    let keys = Arc::new(ApiKeyRegistry::empty());
    let state = Arc::new(RateLimitState::new(keys, config));
    let app = test_app(state);

    // Should never be limited even with limit=1
    for _ in 0..10 {
        let response = app
            .clone()
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
}
