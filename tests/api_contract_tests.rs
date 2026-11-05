use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

use tucano_b3::api::{create_router, AppState};
use tucano_b3::config::B3Settings;
use tucano_b3::db::{create_pool, run_migrations};
use tucano_b3::mcp::tools::*;
use tucano_b3::services::sync_service::B3SyncService;

async fn setup_test_app() -> Option<axum::Router> {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string());

    let pool = match create_pool(&url).await {
        Ok(p) => {
            if run_migrations(&p).await.is_err() {
                return None;
            }
            p
        }
        Err(_) => return None,
    };

    let settings = Arc::new(B3Settings::default());
    let sync_service = Arc::new(B3SyncService::new(pool.clone(), (*settings).clone()));
    let state = Arc::new(AppState {
        pool,
        settings,
        sync_service,
    });

    Some(create_router(state))
}

#[tokio::test]
async fn test_auth_whitelist_and_rejection() {
    let app = match setup_test_app().await {
        Some(a) => a,
        None => {
            eprintln!("Skipping test: Database not available");
            return;
        }
    };

    // 1. Healthz is public
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 2. /api/v1/health is public
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/api/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 3. /docs and /openapi.json are public
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/openapi.json").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(Request::builder().uri("/docs").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. Protected route without auth header returns 401
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/api/v1/b3/instruments").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 5. Protected route with invalid bearer token returns 401
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/b3/instruments")
                .header("Authorization", "Bearer invalid-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 6. Protected route with valid token returns 200
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/b3/instruments")
                .header("Authorization", "Bearer dev-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_openapi_schema_contains_b3_specs_and_disclaimer() {
    let app = match setup_test_app().await {
        Some(a) => a,
        None => {
            eprintln!("Skipping test: Database not available");
            return;
        }
    };

    let res = app
        .oneshot(Request::builder().uri("/openapi.json").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let openapi: Value = serde_json::from_slice(&body_bytes).unwrap();

    let title = openapi["info"]["title"].as_str().unwrap();
    assert!(title.contains("Tucano B3"));

    // Check paths exist
    let paths = &openapi["paths"];
    assert!(paths.get("/api/v1/b3/instruments").is_some());
    assert!(paths.get("/api/v1/b3/cotahist/quotes").is_some());
    assert!(paths.get("/api/v1/b3/price-report/quotes").is_some());

    // Check COTAHIST description warning
    let desc = openapi["info"]["description"].as_str().unwrap();
    assert!(desc.contains("COTAHIST"));
    assert!(desc.contains("preços brutos"));
}

#[tokio::test]
async fn test_cotahist_envelope_includes_disclaimer() {
    let app = match setup_test_app().await {
        Some(a) => a,
        None => {
            eprintln!("Skipping test: Database not available");
            return;
        }
    };

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/b3/cotahist/quotes?limit=10")
                .header("Authorization", "Bearer dev-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json_resp: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(json_resp.get("items").is_some());
    assert!(json_resp.get("total").is_some());
    assert!(json_resp.get("limit").is_some());
    assert_eq!(json_resp["limit"], 10);

    let disclaimer = json_resp["disclaimer"].as_str().unwrap();
    assert!(disclaimer.contains("AVISO: Preços brutos nominais"));
}

#[tokio::test]
async fn test_diagnostics_coverage_endpoint() {
    let app = match setup_test_app().await {
        Some(a) => a,
        None => {
            eprintln!("Skipping test: Database not available");
            return;
        }
    };

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/b3/diagnostics/coverage")
                .header("Authorization", "Bearer dev-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let metrics: Vec<Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert!(!metrics.is_empty());
    assert!(metrics.iter().any(|m| m["dataset"] == "b3_instruments"));
    assert!(metrics.iter().any(|m| m["dataset"] == "b3_cotahist_quotes"));
}

#[tokio::test]
async fn test_mcp_tools_execution() {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/tucano_b3".to_string());

    let pool = match create_pool(&url).await {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Skipping MCP test: Database not reachable");
            return;
        }
    };

    // 1. Healthcheck
    let health = handle_healthcheck(&pool).await;
    assert_eq!(health["status"], "ok");
    assert_eq!(health["database_connected"], true);

    // 2. Search instruments
    let search_args = serde_json::json!({ "query": "PETR4", "limit": 5 });
    let search_res = handle_search_instruments(&pool, &search_args).await;
    assert!(search_res.get("instruments").is_some());

    // 3. Dataset diagnostics
    let diag = handle_get_dataset_diagnostics(&pool).await;
    assert!(diag.get("diagnostics").is_some());
}

