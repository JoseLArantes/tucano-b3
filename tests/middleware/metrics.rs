use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::metrics::AppMetrics;
use tucano_b3::middleware::MetricsLayer;

fn test_app() -> (Router, Arc<AppMetrics>) {
    let metrics = Arc::new(AppMetrics::new());
    let app = Router::new()
        .route("/api/v1/test", get(|| async { "ok" }))
        .route(
            "/api/v1/cotahist/tickers/{ticker}",
            get(|| async { "ticker" }),
        )
        .layer(MetricsLayer::new(metrics.clone()));
    (app, metrics)
}

#[tokio::test]
async fn test_request_counted() {
    let (app, metrics) = test_app();

    let _ = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let output = metrics.render();
    assert!(output.contains("tucano_http_requests_total"));
}

#[tokio::test]
async fn test_duration_recorded() {
    let (app, metrics) = test_app();

    let _ = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let output = metrics.render();
    assert!(output.contains("tucano_http_request_duration_seconds"));
}

#[tokio::test]
async fn test_path_normalization() {
    use tucano_b3::middleware::metrics::normalize_path;

    assert_eq!(
        normalize_path("/api/v1/cotahist/tickers/VALE3"),
        "/api/v1/cotahist/tickers/{ticker}"
    );
    assert_eq!(
        normalize_path("/api/v1/cotahist/dates/2026-01-01/quotes"),
        "/api/v1/cotahist/dates/{date}/quotes"
    );
    assert_eq!(normalize_path("/api/v1/health"), "/api/v1/health");
}

#[tokio::test]
async fn test_status_code_recorded() {
    let (app, metrics) = test_app();

    let _ = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let output = metrics.render();
    assert!(output.contains("status=\"200\""));
}
