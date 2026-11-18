use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn test_health_returns_200() {
    let app = test_app();

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

#[tokio::test]
async fn test_health_returns_json() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

#[tokio::test]
async fn test_health_response_structure() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["status"], "healthy");
    assert!(json["version"].is_string());
    assert!(json["uptime_seconds"].is_number());
    assert!(json["data_files_loaded"].is_number());
    assert!(json["tickers_count"].is_number());
    assert!(json["cache_entries"].is_number());
    assert!(json["cache_enabled"].is_boolean());
}

#[tokio::test]
async fn test_health_response_includes_data_info() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["status"], "healthy");
    assert!(json["data_files_loaded"].is_number());
    assert!(json["tickers_count"].is_number());
    assert!(json["cache_entries"].is_number());
    assert!(json["cache_enabled"].is_boolean());
}

#[tokio::test]
async fn test_info_returns_200() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_info_response_structure() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/info")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["name"], "tucano-b3");
    assert!(json["version"].is_string());
    assert!(json["uptime_seconds"].is_number());
}
