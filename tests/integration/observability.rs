//! Integration tests for observability.

use crate::support::empty_app as test_app;
use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn test_metrics_after_requests() {
    let app = test_app();

    // Make a request to generate metrics
    let _ = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Check metrics endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();

    // Should have at least one request recorded
    assert!(text.contains("tucano_http_requests_total"));
    assert!(text.contains("tucano_uptime_seconds"));
}

#[tokio::test]
async fn test_metrics_not_cached() {
    let app = test_app();

    // Two metrics requests should both return fresh data
    let r1 = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let r2 = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);

    // Neither should have x-cache: HIT
    assert!(r1.headers().get("x-cache").is_none() || r1.headers().get("x-cache").unwrap() != "HIT");
    assert!(r2.headers().get("x-cache").is_none() || r2.headers().get("x-cache").unwrap() != "HIT");
}

#[tokio::test]
async fn test_request_logging_does_not_crash() {
    let app = test_app();

    // Various requests should all succeed without panic
    for path in ["/api/v1/health", "/api/v1/info", "/metrics"] {
        let response = app
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(response.status().is_success());
    }
}
