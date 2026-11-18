use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn test_series_unknown_ticker_returns_404() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ/series?desde=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_series_missing_desde_returns_400() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/series")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_series_invalid_date_returns_400() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/series?desde=not-a-date")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_series_inverted_range_returns_400() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/series?desde=2026-12-31&ate=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_pre_ult_min_unknown_ticker_returns_404() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ/pre-ult/min?desde=2026-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_pre_ult_max_missing_desde_returns_400() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/pre-ult/max")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::BAD_REQUEST
    );
}
