//! Integration tests for rate limiting.

use axum::body::Body;
use axum::http::{Request, StatusCode};

fn test_app_with_rate_limit(limit: u32) -> crate::support::TestApp {
    let mut config = tucano_b3::config::AppConfig::default();
    config.rate_limit.enabled = true;
    config.rate_limit.default_requests_per_minute = limit;
    crate::support::app_with_config(config)
}

#[tokio::test]
async fn test_rate_limit_integration() {
    let app = test_app_with_rate_limit(3);

    // 3 requests should succeed
    for _ in 0..3 {
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/cotahist/tickers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // 4th should be rate limited
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn test_health_bypasses_rate_limit() {
    let app = test_app_with_rate_limit(1);

    // Many health requests should all succeed
    for _ in 0..10 {
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_429_response_format() {
    let app = test_app_with_rate_limit(1);

    // Consume the one token
    let _ = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Get 429
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Verify Retry-After header
    assert!(response.headers().contains_key("retry-after"));

    // Verify body
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
