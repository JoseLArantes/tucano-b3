use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;

use crate::api::schemas::HealthResponse;
use crate::api::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/health",
    tag = "Infrastructure & Health",
    responses(
        (status = 200, description = "Application and database health status", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => "connected".to_string(),
        Err(e) => format!("error: {}", e),
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        app: "tucano-b3".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        database: db_status,
    })
}

pub async fn healthz() -> &'static str {
    "OK"
}

pub async fn worker_health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => "connected".to_string(),
        Err(e) => format!("error: {}", e),
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        app: "tucano-b3-worker".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        database: db_status,
    })
}

