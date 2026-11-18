use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
use tucano_b3::server::build_router;

#[derive(Clone)]
struct TestRouter {
    router: axum::Router,
    _data_dir: Arc<tempfile::TempDir>,
}

impl TestRouter {
    async fn oneshot(
        &self,
        request: Request<Body>,
    ) -> Result<axum::response::Response, std::convert::Infallible> {
        self.router.clone().oneshot(request).await
    }
}

fn test_router() -> TestRouter {
    test_router_with_config(AppConfig::default())
}

fn test_router_with_config(mut config: AppConfig) -> TestRouter {
    let data_dir = Arc::new(tempfile::tempdir().unwrap());
    config.data.dir = data_dir.path().to_path_buf();
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
    TestRouter {
        router,
        _data_dir: data_dir,
    }
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
            "/api/v1/cotahist/tickers/NONEXISTENT99/series?desde=2026-01-01",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99/pre-ult/min?desde=2026-01-01",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99/pre-ult/max?desde=2026-01-01",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99/returns?desde=2026-01-01",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/NONEXISTENT99/stats?desde=2026-01-01",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/dates/2026-01-01/quotes",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/dates/not-a-date/movers",
            StatusCode::BAD_REQUEST,
        ),
        // Avoid full-universe liquidity scan against ./data in this smoke test.
        ("/api/v1/cotahist/liquidity", StatusCode::BAD_REQUEST),
        ("/api/v1/metrics/ibov/history", StatusCode::NOT_FOUND),
        ("/api/v1/metrics/ibov/latest", StatusCode::NOT_FOUND),
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

    for path in ["/api/v1/cotahist/tickers", "/api/v1/metrics/ibov/history"] {
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

    let router = test_router_with_config(config);

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

    // 6. Canonical Swagger UI and machine-readable OpenAPI are public.
    let res_swagger = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/docs/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_swagger.status(), StatusCode::OK);

    let res_openapi = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_openapi.status(), StatusCode::OK);
    let spec = to_bytes(res_openapi.into_body(), usize::MAX).await.unwrap();
    let spec_text = String::from_utf8(spec.to_vec()).unwrap();
    assert!(spec_text.contains("/calendar"));
    assert!(spec_text.contains("/negotiation-hours"));

    // Legacy Swagger / OpenAPI URLs stay public for existing consumers.
    let res_docs = router
        .oneshot(
            Request::builder()
                .uri("/api/docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_docs.status(), StatusCode::OK);
}

#[tokio::test]
async fn live_http_server_serves_seeded_calendar_and_openapi() {
    let test_app = test_router();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = test_app.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let base_url = format!("http://{address}");

    let calendar = reqwest::get(format!("{base_url}/api/v1/calendar/2026"))
        .await
        .unwrap();
    assert_eq!(calendar.status(), StatusCode::OK);
    let calendar: serde_json::Value = calendar.json().await.unwrap();
    assert_eq!(calendar["data"]["ano"], 2026);
    assert_eq!(calendar["data"]["eventos"].as_array().unwrap().len(), 16);

    let openapi = reqwest::get(format!("{base_url}/openapi.json"))
        .await
        .unwrap();
    assert_eq!(openapi.status(), StatusCode::OK);
    let openapi: serde_json::Value = openapi.json().await.unwrap();
    assert!(openapi["paths"].get("/calendar").is_some());
    assert!(openapi["paths"].get("/negotiation-hours").is_some());

    server.abort();
    let _ = server.await;
}
