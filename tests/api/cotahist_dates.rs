use axum::body::Body;
use axum::http::{Request, StatusCode};
fn test_app() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn test_date_quotes_invalid_date() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/dates/not-a-date/quotes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_date_quotes_no_data() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/dates/2030-01-01/quotes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK);
}
