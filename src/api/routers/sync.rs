use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{
    ManualSyncRequest, ManualSyncResponse, PaginatedResponse, PaginationParams,
    RebuildIdentityResponse,
};
use crate::api::AppState;
use crate::db::enums::SourceFileType;
use crate::db::resolver::RelationshipResolver;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceFileItem {
    pub id: String,
    pub source_file_type: String,
    pub source_name: Option<String>,
    pub reference_date: Option<NaiveDate>,
    pub source_url: Option<String>,
    pub original_file_name: Option<String>,
    pub sha256: Option<String>,
    pub byte_size: Option<i64>,
    pub downloaded_at: Option<DateTime<Utc>>,
    pub parsed_at: Option<DateTime<Utc>>,
    pub ingested_at: Option<DateTime<Utc>>,
    pub parser_version: Option<String>,
    pub status: String,
    pub member_count: Option<i32>,
    pub row_count_raw: Option<i32>,
    pub row_count_inserted: Option<i32>,
    pub row_count_changed: Option<i32>,
    pub row_count_unchanged: Option<i32>,
    pub row_count_quarantined: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceFileMemberItem {
    pub id: String,
    pub source_file_id: String,
    pub member_name: String,
    pub member_index: i32,
    pub sha256: String,
    pub byte_size: Option<i64>,
    pub status: String,
    pub row_count_inserted: Option<i32>,
    pub row_count_changed: Option<i32>,
    pub row_count_unchanged: Option<i32>,
    pub processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncRunItem {
    pub id: String,
    pub job_type: String,
    pub requested_by: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: String,
    pub source_file_type: Option<String>,
    pub reference_date_from: Option<NaiveDate>,
    pub reference_date_to: Option<NaiveDate>,
    pub stats_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct SourceFileFilterParams {
    pub source_file_type: Option<String>,
    pub reference_date: Option<NaiveDate>,
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/source-files",
    tag = "B3 Source Files",
    params(PaginationParams, SourceFileFilterParams),
    responses(
        (status = 200, description = "List registered source files", body = PaginatedResponse<SourceFileItem>)
    )
)]
pub async fn list_source_files(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<SourceFileFilterParams>,
) -> Result<Json<PaginatedResponse<SourceFileItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let count_query = r#"
        SELECT count(*)
        FROM b3_source_files
        WHERE ($1::varchar IS NULL OR source_file_type = $1)
          AND ($2::date IS NULL OR reference_date = $2)
          AND ($3::varchar IS NULL OR status = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&filters.source_file_type)
        .bind(filters.reference_date)
        .bind(&filters.status)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            id, source_file_type, source_name, reference_date, source_url,
            original_file_name, sha256, byte_size, downloaded_at, parsed_at,
            ingested_at, parser_version, status, member_count, row_count_raw,
            row_count_inserted, row_count_changed, row_count_unchanged,
            row_count_quarantined, error_message, created_at
        FROM b3_source_files
        WHERE ($1::varchar IS NULL OR source_file_type = $1)
          AND ($2::date IS NULL OR reference_date = $2)
          AND ($3::varchar IS NULL OR status = $3)
        ORDER BY created_at DESC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(&filters.source_file_type)
        .bind(filters.reference_date)
        .bind(&filters.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| SourceFileItem {
            id: r.get("id"),
            source_file_type: r.get("source_file_type"),
            source_name: r.get("source_name"),
            reference_date: r.get("reference_date"),
            source_url: r.get("source_url"),
            original_file_name: r.get("original_file_name"),
            sha256: r.get("sha256"),
            byte_size: r.get("byte_size"),
            downloaded_at: r.get("downloaded_at"),
            parsed_at: r.get("parsed_at"),
            ingested_at: r.get("ingested_at"),
            parser_version: r.get("parser_version"),
            status: r.get("status"),
            member_count: r.get("member_count"),
            row_count_raw: r.get("row_count_raw"),
            row_count_inserted: r.get("row_count_inserted"),
            row_count_changed: r.get("row_count_changed"),
            row_count_unchanged: r.get("row_count_unchanged"),
            row_count_quarantined: r.get("row_count_quarantined"),
            error_message: r.get("error_message"),
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

pub async fn get_source_file_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SourceFileItem>, (StatusCode, String)> {
    let row = sqlx::query(
        r#"
        SELECT 
            id, source_file_type, source_name, reference_date, source_url,
            original_file_name, sha256, byte_size, downloaded_at, parsed_at,
            ingested_at, parser_version, status, member_count, row_count_raw,
            row_count_inserted, row_count_changed, row_count_unchanged,
            row_count_quarantined, error_message, created_at
        FROM b3_source_files
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Source file not found".to_string()))?;

    Ok(Json(SourceFileItem {
        id: row.get("id"),
        source_file_type: row.get("source_file_type"),
        source_name: row.get("source_name"),
        reference_date: row.get("reference_date"),
        source_url: row.get("source_url"),
        original_file_name: row.get("original_file_name"),
        sha256: row.get("sha256"),
        byte_size: row.get("byte_size"),
        downloaded_at: row.get("downloaded_at"),
        parsed_at: row.get("parsed_at"),
        ingested_at: row.get("ingested_at"),
        parser_version: row.get("parser_version"),
        status: row.get("status"),
        member_count: row.get("member_count"),
        row_count_raw: row.get("row_count_raw"),
        row_count_inserted: row.get("row_count_inserted"),
        row_count_changed: row.get("row_count_changed"),
        row_count_unchanged: row.get("row_count_unchanged"),
        row_count_quarantined: row.get("row_count_quarantined"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
    }))
}

pub async fn list_source_file_members(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SourceFileMemberItem>>, (StatusCode, String)> {
    let rows = sqlx::query(
        r#"
        SELECT id, source_file_id, member_name, member_index, sha256, byte_size, status, row_count_inserted, row_count_changed, row_count_unchanged, processed_at
        FROM b3_source_file_members
        WHERE source_file_id = $1
        ORDER BY member_index ASC
        "#,
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| SourceFileMemberItem {
            id: r.get("id"),
            source_file_id: r.get("source_file_id"),
            member_name: r.get("member_name"),
            member_index: r.get("member_index"),
            sha256: r.get("sha256"),
            byte_size: r.get("byte_size"),
            status: r.get("status"),
            row_count_inserted: r.get("row_count_inserted"),
            row_count_changed: r.get("row_count_changed"),
            row_count_unchanged: r.get("row_count_unchanged"),
            processed_at: r.get("processed_at"),
        })
        .collect();

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/sync-runs",
    tag = "B3 Sync Jobs",
    params(PaginationParams),
    responses(
        (status = 200, description = "List sync runs", body = PaginatedResponse<SyncRunItem>)
    )
)]
pub async fn list_sync_runs(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<SyncRunItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_sync_runs")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT id, job_type, requested_by, started_at, finished_at, status, source_file_type, reference_date_from, reference_date_to, stats_json, created_at
        FROM b3_sync_runs
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
        .map(|r| SyncRunItem {
            id: r.get("id"),
            job_type: r.get("job_type"),
            requested_by: r.get("requested_by"),
            started_at: r.get("started_at"),
            finished_at: r.get("finished_at"),
            status: r.get("status"),
            source_file_type: r.get("source_file_type"),
            reference_date_from: r.get("reference_date_from"),
            reference_date_to: r.get("reference_date_to"),
            stats_json: r.get("stats_json"),
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

pub async fn get_sync_run_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SyncRunItem>, (StatusCode, String)> {
    let row = sqlx::query(
        r#"
        SELECT id, job_type, requested_by, started_at, finished_at, status, source_file_type, reference_date_from, reference_date_to, stats_json, created_at
        FROM b3_sync_runs
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Sync run not found".to_string()))?;

    Ok(Json(SyncRunItem {
        id: row.get("id"),
        job_type: row.get("job_type"),
        requested_by: row.get("requested_by"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        status: row.get("status"),
        source_file_type: row.get("source_file_type"),
        reference_date_from: row.get("reference_date_from"),
        reference_date_to: row.get("reference_date_to"),
        stats_json: row.get("stats_json"),
        created_at: row.get("created_at"),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/b3/sync-runs",
    tag = "B3 Sync Jobs",
    request_body = ManualSyncRequest,
    responses(
        (status = 200, description = "Manual sync executed", body = ManualSyncResponse),
        (status = 400, description = "Invalid request arguments"),
        (status = 409, description = "Sync run lock conflict (concurrent sync already running)")
    )
)]
pub async fn trigger_sync_run(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ManualSyncRequest>,
) -> Result<Json<ManualSyncResponse>, (StatusCode, String)> {
    let sft = SourceFileType::from_str(&payload.source_file_type).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid source_file_type: {e}"),
        )
    })?;

    let force_dl = payload.force_download.unwrap_or(false);
    let force_rp = payload.force_reparse.unwrap_or(false);

    let outcome = state
        .sync_service
        .run_manual_sync(
            sft,
            payload.reference_date_from,
            payload.reference_date_to,
            payload.source_url,
            None,
            force_dl,
            force_rp,
            Some("api_user".to_string()),
        )
        .await
        .map_err(|err| match err {
            crate::error::AppError::LockConflict(msg) => (StatusCode::CONFLICT, msg),
            crate::error::AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    Ok(Json(ManualSyncResponse {
        sync_run_id: outcome.sync_run_id,
        status: outcome.status.to_string(),
        inserted: outcome.inserted,
        changed: outcome.changed,
        unchanged: outcome.unchanged,
        unresolved: outcome.unresolved,
        warnings: outcome.warnings,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/b3/sync-runs/rebuild-identity",
    tag = "B3 Sync Jobs",
    responses(
        (status = 200, description = "Identity resolution rebuild completed", body = RebuildIdentityResponse)
    )
)]
pub async fn rebuild_identity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RebuildIdentityResponse>, (StatusCode, String)> {
    let resolver = RelationshipResolver::new(&state.pool);
    let resolved = resolver
        .rebuild_unresolved_identities()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RebuildIdentityResponse {
        resolved_count: resolved as i32,
        message: format!(
            "Rebuilt identity relationships; resolved {resolved} previously pending items."
        ),
    }))
}
