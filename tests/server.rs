use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::config::AppConfig;
use tucano_b3::data::reload::DataLoader;
use tucano_b3::server::build_router;

#[tokio::test]
async fn test_build_router_succeeds() {
    let config = AppConfig::default();
    let data_loader = Arc::new(DataLoader::new(&config.data).unwrap());
    let router = build_router(&config, data_loader);
    drop(router);
}

#[tokio::test]
async fn test_cotahist_routes_registered() {
    let config = AppConfig::default();
    let data_loader = Arc::new(DataLoader::new(&config.data).unwrap());
    let router = build_router(&config, data_loader);

    let test_routes = vec![
        ("/api/v1/cotahist/tickers", StatusCode::OK),
        ("/api/v1/cotahist/tickers/VALE3", StatusCode::NOT_FOUND),
        (
            "/api/v1/cotahist/tickers/VALE3/quotes",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/tickers/VALE3/latest",
            StatusCode::NOT_FOUND,
        ),
        (
            "/api/v1/cotahist/dates/2026-01-01/quotes",
            StatusCode::NOT_FOUND,
        ),
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
async fn test_cotahist_method_not_allowed() {
    let config = AppConfig::default();
    let data_loader = Arc::new(DataLoader::new(&config.data).unwrap());
    let router = build_router(&config, data_loader);

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .expect("failed to build request"),
        )
        .await
        .expect("request failed");

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
