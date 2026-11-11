use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::cache::store::CacheStore;
use tucano_b3::config::CacheConfig;
use tucano_b3::middleware::CacheLayer;

fn test_cache() -> Arc<CacheStore> {
    let config = CacheConfig {
        enabled: true,
        ttl_seconds: 300,
        max_entries: 100,
    };
    Arc::new(CacheStore::new(&config))
}

fn test_app(cache: Arc<CacheStore>) -> Router {
    Router::new()
        .route("/api/v1/test", get(|| async { "response_data" }))
        .layer(CacheLayer::new(cache))
}

#[tokio::test]
async fn test_cache_miss_then_hit() {
    let cache = test_cache();
    let app = test_app(cache.clone());

    // First request: MISS
    let response1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response1.status(), 200);
    let x_cache = response1
        .headers()
        .get("x-cache")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(x_cache, "MISS");

    // Second request: HIT
    let response2 = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response2.status(), 200);
    let x_cache = response2
        .headers()
        .get("x-cache")
        .unwrap()
        .to_str()
        .unwrap();
    assert_eq!(x_cache, "HIT");
}

#[tokio::test]
async fn test_cache_disabled() {
    let cache = Arc::new(CacheStore::disabled());
    let app = test_app(cache);

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
    assert!(response.headers().get("x-cache").is_none());
}

#[tokio::test]
async fn test_health_not_cached() {
    let cache = test_cache();
    let app = Router::new()
        .route("/api/v1/health", get(|| async { "healthy" }))
        .layer(CacheLayer::new(cache.clone()));

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let _ = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(cache.len(), 0);
}
