use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn test_latest_unknown_ticker_returns_404() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error"]["code"], "TICKER_NOT_FOUND");
}
