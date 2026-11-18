use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

async fn status_for(path: &str) -> StatusCode {
    let app = test_app();
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    response.status()
}

#[tokio::test]
async fn test_analytics_unknown_ticker_returns_404() {
    for path in [
        "/api/v1/cotahist/tickers/XYZXYZ/returns?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/stats?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/drawdown?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/range-position?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/relative?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/gaps?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/rolling?desde=2026-01-01",
        "/api/v1/cotahist/tickers/XYZXYZ/spread?desde=2026-01-01",
    ] {
        assert_eq!(status_for(path).await, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn test_analytics_missing_desde_returns_400_or_404() {
    for path in [
        "/api/v1/cotahist/tickers/VALE3/returns",
        "/api/v1/cotahist/tickers/VALE3/stats",
        "/api/v1/cotahist/tickers/VALE3/drawdown",
        "/api/v1/cotahist/liquidity",
        "/api/v1/cotahist/compare?tickers=VALE3",
    ] {
        let status = status_for(path).await;
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
            "{path} -> {status}"
        );
    }
}

#[tokio::test]
async fn test_movers_invalid_ranking_returns_400_or_404() {
    let status = status_for("/api/v1/cotahist/dates/2026-01-02/movers?ranking=nope").await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::NOT_FOUND,
        "unexpected {status}"
    );
}

#[tokio::test]
async fn test_compare_route_registered() {
    let status = status_for("/api/v1/cotahist/compare?tickers=XYZXYZ&desde=2026-01-01").await;
    assert_ne!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST,
        "unexpected {status}"
    );
}
