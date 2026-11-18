use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn test_metrics_returns_200() {
    let app = test_app();
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
}

#[tokio::test]
async fn test_metrics_content_type() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/plain"));
}

#[tokio::test]
async fn test_metrics_contains_tucano_metrics() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();

    assert!(text.contains("tucano_http_requests_total"));
    assert!(text.contains("tucano_uptime_seconds"));
    assert!(text.contains("tucano_service_info"));
    assert!(text.contains("tucano_data_files_loaded"));
    assert!(text.contains("tucano_cache_entries"));
}

#[tokio::test]
async fn test_metrics_not_rate_limited() {
    let app = test_app();

    // Make many metrics requests — should never be limited
    for _ in 0..20 {
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
    }
}
