use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
use tucano_b3::server::build_router;

fn test_router() -> axum::Router {
    let config = AppConfig::default();
    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let data_loader = Arc::new(DataLoader::new(&config.data, cache.clone()).unwrap());
    let api_keys = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    let metrics = Arc::new(tucano_b3::metrics::AppMetrics::new());
    let start_time = std::time::Instant::now();
    build_router(
        &config,
        data_loader,
        cache,
        rate_limit_state,
        metrics,
        start_time,
    )
}

#[tokio::test]
async fn test_build_router_succeeds() {
    let router = test_router();
    drop(router);
}

#[tokio::test]
async fn test_cotahist_and_ibov_routes_registered() {
    let router = test_router();

    let test_routes = vec![
        ("/api/v1/cotahist/tickers", StatusCode::OK),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99/quotes",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99/latest",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/dates/2026-01-01/quotes",
            StatusCode::NOT_FOUND,
        ),
        ("/api/v1/ibov/history", StatusCode::NOT_FOUND),
        ("/api/v1/ibov/latest", StatusCode::NOT_FOUND),
    ];

    for (path, expected_status) in test_routes {
        let app = router.clone();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(path)
                    .body(Body::empty())
                    .expect("failed to build request"),
            )
            .await
            .expect("request failed");

        assert_eq!(
            response.status(),
            expected_status,
            "Route {} returned unexpected status {}",
            path,
            response.status()
        );
        // Ensure not 405 Method Not Allowed
        assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}

#[tokio::test]
async fn test_method_not_allowed() {
    let router = test_router();

    for path in ["/api/v1/cotahist/tickers", "/api/v1/ibov/history"] {
        let app = router.clone();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .body(Body::empty())
                    .expect("failed to build request"),
            )
            .await
            .expect("request failed");

        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "Route {} should reject POST with 405",
            path
        );
    }
}
