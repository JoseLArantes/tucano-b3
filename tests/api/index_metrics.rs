use axum::body::Body;
use axum::http::{Request, StatusCode};
fn router() -> crate::support::TestApp {
    crate::support::empty_app()
}

#[tokio::test]
async fn index_ingestion_status_requires_configured_authentication() {
    let response = router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/ingest/indices/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn old_ibov_routes_are_removed() {
    for path in ["/api/v1/ibov/history", "/api/v1/ibov/latest"] {
        let response = router()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
