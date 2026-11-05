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

#[utoipa::path(
    get,
    path = "/metrics",
    tag = "Infrastructure & Health",
    responses(
        (status = 200, description = "Prometheus text format metrics export")
    )
)]
pub async fn metrics(State(state): State<Arc<AppState>>) -> String {
    let instruments_count: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_instruments")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let source_files_count: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_source_files")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let sync_runs_count: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_sync_runs")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let field_changes_count: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_field_changes")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    let quarantine_count: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_quarantine_records")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    format!(
        "# HELP tucano_b3_up Whether the service is running\n\
         # TYPE tucano_b3_up gauge\n\
         tucano_b3_up 1\n\
         # HELP tucano_b3_instruments_total Total instruments stored\n\
         # TYPE tucano_b3_instruments_total gauge\n\
         tucano_b3_instruments_total {}\n\
         # HELP tucano_b3_source_files_total Total source files processed\n\
         # TYPE tucano_b3_source_files_total counter\n\
         tucano_b3_source_files_total {}\n\
         # HELP tucano_b3_sync_runs_total Total sync runs executed\n\
         # TYPE tucano_b3_sync_runs_total counter\n\
         tucano_b3_sync_runs_total {}\n\
         # HELP tucano_b3_field_changes_total Total field change audit records\n\
         # TYPE tucano_b3_field_changes_total counter\n\
         tucano_b3_field_changes_total {}\n\
         # HELP tucano_b3_quarantine_records_total Total quarantined records\n\
         # TYPE tucano_b3_quarantine_records_total counter\n\
         tucano_b3_quarantine_records_total {}\n",
        instruments_count,
        source_files_count,
        sync_runs_count,
        field_changes_count,
        quarantine_count
    )
}
