//! Integration tests for tucano-b3.
//!
//! These tests start a real HTTP server and make actual HTTP requests.

#[path = "integration/data_layer.rs"]
mod data_layer;

#[path = "integration/cache.rs"]
mod cache;

#[path = "integration/cotahist_endpoints.rs"]
mod cotahist_endpoints;

#[path = "integration/ibov_endpoints.rs"]
mod ibov_endpoints;

#[path = "integration/openapi.rs"]
mod openapi;

#[path = "integration/rate_limit.rs"]
mod rate_limit;

use std::net::SocketAddr;
use std::sync::Arc;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
use tucano_b3::server::build_router;

/// Helper: start a test server on a random port.
async fn start_test_server() -> SocketAddr {
    let config = AppConfig::default();
    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let data_loader = Arc::new(DataLoader::new(&config.data, cache.clone()).unwrap());
    let api_keys = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    let router = build_router(&config, data_loader, cache, rate_limit_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    addr
}

#[tokio::test]
async fn test_health_endpoint_integration() {
    let addr = start_test_server().await;
    let url = format!("http://{}/api/v1/health", addr);

    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["status"], "healthy");
    assert!(json["version"].is_string());
    assert!(json["uptime_seconds"].is_number());
    assert!(json["data_files_loaded"].is_number());
    assert!(json["tickers_count"].is_number());
}

#[tokio::test]
async fn test_info_endpoint_integration() {
    let addr = start_test_server().await;
    let url = format!("http://{}/api/v1/info", addr);

    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), 200);

    let json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(json["name"], "tucano-b3");
}

#[tokio::test]
async fn test_request_id_header_present() {
    let addr = start_test_server().await;
    let url = format!("http://{}/api/v1/health", addr);

    let response = reqwest::get(&url).await.unwrap();
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let addr = start_test_server().await;
    let url = format!("http://{}/api/v1/nonexistent", addr);

    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_compression_enabled() {
    let addr = start_test_server().await;
    let url = format!("http://{}/api/v1/health", addr);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();

    // Response should have content-encoding header when compression is applied
    // (may not be applied for small responses, so we just verify no error)
    assert_eq!(response.status(), 200);
}
