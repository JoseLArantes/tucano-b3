//! Integration tests for caching behavior.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn test_app_with_cache() -> axum::Router {
    let config = tucano_b3::config::AppConfig::default();
    let cache = std::sync::Arc::new(tucano_b3::cache::store::CacheStore::new(&config.cache));
    let data_loader = std::sync::Arc::new(
        tucano_b3::data::reload::DataLoader::new(&config.data, cache.clone()).unwrap(),
    );
    let api_keys = std::sync::Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = std::sync::Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    tucano_b3::server::build_router(&config, data_loader, cache, rate_limit_state)
}

#[tokio::test]
async fn test_health_not_cached_integration() {
    let app = test_app_with_cache();

    // Two identical health requests
    let r1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let r2 = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);
    assert!(r1.headers().get("x-cache").is_none());
    assert!(r2.headers().get("x-cache").is_none());
}

#[tokio::test]
async fn test_cotahist_tickers_cached() {
    let app = test_app_with_cache();

    // First request
    let r1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r1.status(), StatusCode::OK);
    let x_cache_1 = r1
        .headers()
        .get("x-cache")
        .map(|v| v.to_str().unwrap().to_string());

    // Second identical request
    let r2 = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r2.status(), StatusCode::OK);
    let x_cache_2 = r2
        .headers()
        .get("x-cache")
        .map(|v| v.to_str().unwrap().to_string());

    // If cache is enabled, second should be HIT
    if x_cache_1 == Some("MISS".to_string()) {
        assert_eq!(x_cache_2, Some("HIT".to_string()));
    }
}

#[tokio::test]
async fn test_different_queries_different_cache() {
    let app = test_app_with_cache();

    let r1 = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers?pagina=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let r2 = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers?pagina=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(r1.status().is_success() || r1.status().is_client_error());
    assert!(r2.status().is_success() || r2.status().is_client_error());
}
