//! Integration tests for ingestion endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let config = tucano_b3::config::AppConfig::default();
    let cache = std::sync::Arc::new(tucano_b3::cache::store::CacheStore::new(&config.cache));
    let data_loader = std::sync::Arc::new(
        tucano_b3::data::reload::DataLoader::new(&config.data, cache.clone()).unwrap(),
    );
    let api_keys =
        std::sync::Arc::new(tucano_b3::middleware::rate_limit::keys::ApiKeyRegistry::empty());
    let rate_state = std::sync::Arc::new(
        tucano_b3::middleware::rate_limit::layer::RateLimitState::new(
            api_keys,
            config.rate_limit.clone(),
        ),
    );
    let metrics = std::sync::Arc::new(tucano_b3::metrics::AppMetrics::new());
    let start_time = std::time::Instant::now();

    tucano_b3::server::build_router(&config, data_loader, cache, rate_state, metrics, start_time)
}

#[tokio::test]
async fn test_upload_without_file_returns_400() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/upload")
                .header("content-type", "multipart/form-data; boundary=test")
                .body(Body::from("--test--"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should be 400 (bad request) not 405 (method not allowed)
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_upload_over_default_body_limit_is_not_rejected() {
    let app = test_app();

    // Axum's default body limit is 2 MB. COTAHIST ZIPs are larger than that.
    let payload = vec![b'x'; 2 * 1024 * 1024 + 64 * 1024];
    let mut body = Vec::new();
    body.extend_from_slice(
        b"--test\r\nContent-Disposition: form-data; name=\"file\"; filename=\"COTAHIST.zip\"\r\nContent-Type: application/zip\r\n\r\n",
    );
    body.extend_from_slice(&payload);
    body.extend_from_slice(b"\r\n--test--\r\n");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/upload")
                .header("content-type", "multipart/form-data; boundary=test")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(response.status().is_client_error() || response.status().is_server_error());
}

#[tokio::test]
async fn test_folder_not_found_returns_error() {
    let app = test_app();

    let body = serde_json::json!({
        "caminho": "/nonexistent/folder"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/folder")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_ingest_endpoints_in_openapi() {
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
    assert!(paths.contains_key("/ingest/upload"));
    assert!(paths.contains_key("/ingest/folder"));
    assert!(paths.contains_key("/ingest/bucket"));
}

#[tokio::test]
async fn test_bucket_without_configuration_returns_400() {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/bucket")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let message = json["error"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("TUCANO_S3_ENDPOINT"));
}
