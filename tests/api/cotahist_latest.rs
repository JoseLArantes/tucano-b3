use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let config = tucano_b3::config::AppConfig::default();
    let data_loader = Arc::new(tucano_b3::data::reload::DataLoader::new(&config.data).unwrap());
    tucano_b3::server::build_router(&config, data_loader)
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
