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
async fn test_list_tickers_empty() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_tickers_with_search() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers?busca=VALE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_unknown_ticker_returns_404() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_tickers_pagination_params() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers?pagina=1&por_pagina=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
