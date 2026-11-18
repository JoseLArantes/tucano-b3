//! Integration tests for caching behavior.

use crate::support::empty_app as test_app_with_cache;
use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn test_health_not_cached_integration() {
    let app = test_app_with_cache();

    // Two identical health requests
    let r1 = app
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
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r1.headers()["x-cache"], "MISS");

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
    assert_eq!(r2.headers()["x-cache"], "HIT");
}

#[tokio::test]
async fn test_different_queries_different_cache() {
    let app = test_app_with_cache();

    let r1 = app
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

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);
    assert_eq!(r1.headers()["x-cache"], "MISS");
    assert_eq!(r2.headers()["x-cache"], "MISS");
}
