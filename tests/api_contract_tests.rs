use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower::ServiceExt;
use tucano_b3::{
    api::{create_router, AppState},
    config::B3Settings,
    services::sync_service::B3SyncService,
};

fn app() -> axum::Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unreachable_test")
        .unwrap();
    let settings = Arc::new(B3Settings {
        tucano_b3_read_token: "test-read".into(),
        tucano_b3_admin_token: "test-admin".into(),
        ..B3Settings::default()
    });
    create_router(Arc::new(AppState {
        pool: pool.clone(),
        settings: settings.clone(),
        sync_service: Arc::new(B3SyncService::new(pool, (*settings).clone())),
    }))
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), 1_048_576)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn public_exceptions_and_read_admin_separation_are_explicit() {
    for path in ["/healthz", "/openapi.json"] {
        assert_eq!(
            app()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/b3/cotahist/quotes")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/b3/cotahist/quotes/export?ticker_symbol=EXMP3")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for path in [
        "/api/v1/b3/jobs",
        "/api/v1/b3/source-files",
        "/api/v1/b3/pending-updates",
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("Authorization", "Bearer test-read")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body(response).await["code"], "ADMIN_REQUIRED");
    }
    let response = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_pagination_and_unknown_parameters_are_typed_before_queries() {
    for suffix in [
        "limit=0",
        "limit=1001",
        "offset=-1",
        "trade_date_from=2026-02-01&trade_date_to=2026-01-01",
        "raw_sql=DELETE",
    ] {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/b3/cotahist/quotes?{suffix}"))
                    .header("Authorization", "Bearer test-read")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{suffix}");
        let value = body(response).await;
        assert_eq!(value["code"], "INVALID_FILTER");
        assert!(value["request_id"].is_string());
        assert!(!value.to_string().contains("postgres"));
    }
}

#[tokio::test]
async fn administrative_json_is_closed_and_never_executes_inline() {
    let response=app().oneshot(Request::builder().method("POST").uri("/api/v1/b3/jobs").header("Authorization","Bearer test-admin").header("content-type","application/json").body(Body::from(json!({"idempotency_key":"example","command":{"operation":"publish_revision","revision_id":uuid::Uuid::new_v4(),"acknowledge_issues":false,"sql":"DELETE"},"priority":"maintenance"}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(response).await["code"], "INVALID_REQUEST");
}

#[tokio::test]
async fn openapi_describes_only_supported_release_contracts() {
    let spec = body(
        app()
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(spec["openapi"], "3.1.0");
    assert!(spec["info"]["description"]
        .as_str()
        .unwrap()
        .contains("não ajustados"));
    for path in [
        "/api/v1/b3/cotahist/quotes",
        "/api/v1/b3/cotahist/quotes/export",
        "/api/v1/b3/cotahist/series",
        "/api/v1/b3/instruments/{id}",
        "/api/v1/b3/instruments/{id}/identifiers",
        "/api/v1/b3/tradable-securities",
        "/api/v1/b3/jobs",
    ] {
        assert!(spec["paths"].get(path).is_some());
    }
    assert!(spec["paths"]
        .get("/api/v1/b3/index-composition/portfolios")
        .is_none());
    assert!(spec["paths"]["/api/v1/b3/jobs"]["post"]["responses"]
        .get("202")
        .is_some());
    let export = &spec["paths"]["/api/v1/b3/cotahist/quotes/export"]["get"];
    assert_eq!(
        export["responses"]["200"]["content"]["application/x-ndjson"]["schema"]["$ref"],
        "#/components/schemas/QuoteExportEvent"
    );
    assert!(export["responses"].get("304").is_some());
    assert!(export["security"][0].get("bearer_auth").is_some());
    assert!(spec["components"]["schemas"]["QuoteExportFilters"].is_object());

    let description = export["description"].as_str().unwrap();
    for required_detail in [
        "publicações oficiais COTAHIST",
        "Retomada e idempotência",
        "content_sha256",
        "third_party_delivery",
        "500 fatos",
        "application/x-ndjson",
    ] {
        assert!(
            description.contains(required_detail),
            "missing OpenAPI detail: {required_detail}"
        );
    }
    let parameters = export["parameters"].as_array().unwrap();
    for name in [
        "ticker_symbol",
        "data_revision",
        "cursor",
        "include_internal",
        "If-None-Match",
        "Accept-Encoding",
    ] {
        let parameter = parameters
            .iter()
            .find(|parameter| parameter["name"] == name)
            .unwrap_or_else(|| panic!("missing OpenAPI parameter: {name}"));
        assert!(parameter["description"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
    }
    let cursor = parameters
        .iter()
        .find(|parameter| parameter["name"] == "cursor")
        .unwrap();
    assert_eq!(cursor["schema"]["maxLength"], 1024);
    assert!(cursor["example"].is_string() || cursor["schema"]["example"].is_string());
}
