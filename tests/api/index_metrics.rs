use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

fn router() -> axum::Router {
    let config = tucano_b3::config::AppConfig::default();
    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let loader =
        Arc::new(tucano_b3::data::reload::DataLoader::new(&config.data, cache.clone()).unwrap());
    let registry = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate = Arc::new(tucano_b3::middleware::RateLimitState::new(
        registry,
        config.rate_limit.clone(),
    ));
    tucano_b3::server::build_router(
        &config,
        loader,
        cache,
        rate,
        Arc::new(tucano_b3::metrics::AppMetrics::new()),
        std::time::Instant::now(),
    )
}

#[tokio::test]
async fn index_ingestion_status_requires_configured_authentication() {
    let response = router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ingest/indices/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn old_ibov_routes_are_removed() {
    for path in ["/api/v1/ibov/history", "/api/v1/ibov/latest"] {
        let response = router()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
