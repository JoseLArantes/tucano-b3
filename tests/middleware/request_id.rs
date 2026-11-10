use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use tower::ServiceExt;
use tucano_b3::middleware::request_id::REQUEST_ID_HEADER;
use tucano_b3::middleware::RequestIdLayer;

fn test_app() -> Router {
    Router::new()
        .route("/test", get(|| async { "ok" }))
        .layer(RequestIdLayer::new())
}

#[tokio::test]
async fn test_request_id_generated() {
    let app = test_app();

    let response = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let request_id = response.headers().get(REQUEST_ID_HEADER);
    assert!(request_id.is_some());

    let request_id_str = request_id.unwrap().to_str().unwrap();
    // UUID v4 format: 8-4-4-4-12 hex chars
    assert_eq!(request_id_str.len(), 36);
    assert_eq!(request_id_str.chars().filter(|c| *c == '-').count(), 4);
}

#[tokio::test]
async fn test_request_id_preserved_from_header() {
    let app = test_app();
    let custom_id = "my-custom-request-id";

    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header(REQUEST_ID_HEADER, custom_id)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let request_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
    assert_eq!(request_id.to_str().unwrap(), custom_id);
}

#[tokio::test]
async fn test_each_request_gets_unique_id() {
    let app = test_app();

    let response1 = app
        .clone()
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let response2 = app
        .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let id1 = response1
        .headers()
        .get(REQUEST_ID_HEADER)
        .unwrap()
        .to_str()
        .unwrap();
    let id2 = response2
        .headers()
        .get(REQUEST_ID_HEADER)
        .unwrap()
        .to_str()
        .unwrap();

    assert_ne!(id1, id2);
}
