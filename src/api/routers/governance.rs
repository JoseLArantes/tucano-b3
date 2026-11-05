use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{CoverageMetric, PaginatedResponse, PaginationParams, QuarantineReplayResponse};
use crate::api::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuarantineItem {
    pub id: String,
    pub sync_run_id: Option<String>,
    pub source_file_id: Option<String>,
    pub source_member_name: Option<String>,
    pub source_file_type: String,
    pub reference_date: Option<NaiveDate>,
    pub source_line_number: Option<i32>,
    pub reason_code: String,
    pub error_message: String,
    pub raw_payload: serde_json::Value,
    pub resolution_status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldChangeItem {
    pub id: String,
    pub sync_run_id: Option<String>,
    pub source_file_id: Option<String>,
    pub table_name: String,
    pub record_pk: String,
    pub field_name: String,
    pub old_value_text: Option<String>,
    pub new_value_text: Option<String>,
    pub changed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingUpdateItem {
    pub id: String,
    pub source_file_type: String,
    pub reference_date: NaiveDate,
    pub source_url: Option<String>,
    pub remote_etag: Option<String>,
    pub status: String,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct FieldChangeFilterParams {
    pub table_name: Option<String>,
    pub record_pk: Option<String>,
    pub field_name: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/quarantine",
    tag = "B3 Data Quality & Governance",
    params(PaginationParams),
    responses(
        (status = 200, description = "List quarantined records", body = PaginatedResponse<QuarantineItem>)
    )
)]
pub async fn list_quarantine_records(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<QuarantineItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_quarantine_records")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT id, sync_run_id, source_file_id, source_member_name, source_file_type, reference_date, source_line_number, reason_code, error_message, raw_payload, resolution_status, created_at
        FROM b3_quarantine_records
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| QuarantineItem {
            id: r.get("id"),
            sync_run_id: r.get("sync_run_id"),
            source_file_id: r.get("source_file_id"),
            source_member_name: r.get("source_member_name"),
            source_file_type: r.get("source_file_type"),
            reference_date: r.get("reference_date"),
            source_line_number: r.get("source_line_number"),
            reason_code: r.get("reason_code"),
            error_message: r.get("error_message"),
            raw_payload: r.get("raw_payload"),
            resolution_status: r.get("resolution_status"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
        disclaimer: None,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/b3/quarantine/replay",
    tag = "B3 Data Quality & Governance",
    responses(
        (status = 200, description = "Replay quarantine records", body = QuarantineReplayResponse)
    )
)]
pub async fn replay_quarantine(
    State(state): State<Arc<AppState>>,
) -> Result<Json<QuarantineReplayResponse>, (StatusCode, String)> {
    let rows = sqlx::query(
        r#"
        UPDATE b3_quarantine_records
        SET resolution_status = 'reprocessed', reprocessed_at = NOW()
        WHERE resolution_status = 'quarantined'
        RETURNING id
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let count = rows.len() as i32;
    Ok(Json(QuarantineReplayResponse {
        reprocessed_count: count,
        message: format!("Replayed {count} quarantine records."),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/field-changes",
    tag = "B3 Data Quality & Governance",
    params(PaginationParams, FieldChangeFilterParams),
    responses(
        (status = 200, description = "List changed fields audit log", body = PaginatedResponse<FieldChangeItem>)
    )
)]
pub async fn list_field_changes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<FieldChangeFilterParams>,
) -> Result<Json<PaginatedResponse<FieldChangeItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let count_query = r#"
        SELECT count(*)
        FROM b3_field_changes
        WHERE ($1::varchar IS NULL OR table_name = $1)
          AND ($2::varchar IS NULL OR record_pk = $2)
          AND ($3::varchar IS NULL OR field_name = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&filters.table_name)
        .bind(&filters.record_pk)
        .bind(&filters.field_name)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT id, sync_run_id, source_file_id, table_name, record_pk, field_name, old_value_text, new_value_text, changed_at
        FROM b3_field_changes
        WHERE ($1::varchar IS NULL OR table_name = $1)
          AND ($2::varchar IS NULL OR record_pk = $2)
          AND ($3::varchar IS NULL OR field_name = $3)
        ORDER BY changed_at DESC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(&filters.table_name)
        .bind(&filters.record_pk)
        .bind(&filters.field_name)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| FieldChangeItem {
            id: r.get("id"),
            sync_run_id: r.get("sync_run_id"),
            source_file_id: r.get("source_file_id"),
            table_name: r.get("table_name"),
            record_pk: r.get("record_pk"),
            field_name: r.get("field_name"),
            old_value_text: r.get("old_value_text"),
            new_value_text: r.get("new_value_text"),
            changed_at: r.get("changed_at"),
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
        disclaimer: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/pending-updates",
    tag = "B3 Data Quality & Governance",
    params(PaginationParams),
    responses(
        (status = 200, description = "List pending updates discovered by scanner", body = PaginatedResponse<PendingUpdateItem>)
    )
)]
pub async fn list_pending_updates(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PendingUpdateItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_pending_updates")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT id, source_file_type, reference_date, source_url, remote_etag, status, discovered_at
        FROM b3_pending_updates
        ORDER BY reference_date DESC, discovered_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| PendingUpdateItem {
            id: r.get("id"),
            source_file_type: r.get("source_file_type"),
            reference_date: r.get("reference_date"),
            source_url: r.get("source_url"),
            remote_etag: r.get("remote_etag"),
            status: r.get("status"),
            discovered_at: r.get("discovered_at"),
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
        disclaimer: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/diagnostics/coverage",
    tag = "B3 Data Quality & Governance",
    responses(
        (status = 200, description = "Dataset coverage and date boundaries diagnostics", body = Vec<CoverageMetric>)
    )
)]
pub async fn get_coverage_diagnostics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CoverageMetric>>, (StatusCode, String)> {
    let queries = vec![
        ("b3_instruments", "SELECT min(created_at::date), max(created_at::date), count(*) FROM b3_instruments"),
        ("b3_tradable_securities", "SELECT min(reference_date), max(reference_date), count(*) FROM b3_tradable_securities"),
        ("b3_price_report_quotes", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_price_report_quotes"),
        ("b3_equities_simplified_price_quotes", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_equities_simplified_price_quotes"),
        ("b3_index_report_values", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_index_report_values"),
        ("b3_index_portfolio_compositions", "SELECT min(composition_date), max(composition_date), count(*) FROM b3_index_portfolio_compositions"),
        ("b3_cotahist_quotes", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_cotahist_quotes"),
    ];

    let mut metrics = Vec::new();

    for (dataset, q) in queries {
        let row = sqlx::query(q)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let min_d: Option<NaiveDate> = row.get(0);
        let max_d: Option<NaiveDate> = row.get(1);
        let total: i64 = row.get(2);

        let files_count: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_source_files WHERE status = 'ingested'")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

        metrics.push(CoverageMetric {
            dataset: dataset.to_string(),
            min_date: min_d,
            max_date: max_d,
            total_records: total,
            total_files: files_count,
        });
    }

    Ok(Json(metrics))
}

