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

#[tokio::test]
async fn test_redoc_endpoints_return_200() {
    let router = test_router();

    for path in ["/redoc", "/api/redoc"] {
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

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&bytes);
        assert!(html.contains("redoc"));
        assert!(html.contains("/api/docs/openapi.json"));
    }
}

#[tokio::test]
async fn test_health_root_returns_200() {
    let router = test_router();

    let response = router
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protected_endpoints_require_token_when_configured() {
    let mut config = AppConfig::default();
    config.auth.token = Some("secret-test-token".to_string());

    let cache = Arc::new(tucano_b3::cache::CacheStore::new(&config.cache));
    let data_loader = Arc::new(DataLoader::new(&config.data, cache.clone()).unwrap());
    let api_keys = Arc::new(tucano_b3::middleware::ApiKeyRegistry::empty());
    let rate_limit_state = Arc::new(tucano_b3::middleware::RateLimitState::new(
        api_keys,
        config.rate_limit.clone(),
    ));
    let metrics = Arc::new(tucano_b3::metrics::AppMetrics::new());
    let start_time = std::time::Instant::now();

    let router = build_router(
        &config,
        data_loader,
        cache,
        rate_limit_state,
        metrics,
        start_time,
    );

    // 1. Without token -> 401 Unauthorized
    let res_no_token = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_no_token.status(), StatusCode::UNAUTHORIZED);

    // 2. With invalid token -> 401 Unauthorized
    let res_invalid = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("Authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_invalid.status(), StatusCode::UNAUTHORIZED);

    // 3. With valid Bearer token -> 200 OK
    let res_bearer = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("Authorization", "Bearer secret-test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_bearer.status(), StatusCode::OK);

    // 4. With valid X-API-Key -> 200 OK
    let res_api_key = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .header("X-API-Key", "secret-test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_api_key.status(), StatusCode::OK);

    // 5. Health bypasses token -> 200 OK
    let res_health = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_health.status(), StatusCode::OK);

    // 6. Redoc bypasses token -> 200 OK
    let res_redoc = router
        .oneshot(
            Request::builder()
                .uri("/redoc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_redoc.status(), StatusCode::OK);
}
