use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{InternalMetadata, PaginatedResponse, PaginationParams, Provenance};
use crate::api::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexItem {
    pub id: String,
    pub index_code: String,
    pub index_name: Option<String>,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexValueItem {
    pub id: String,
    pub index_id: Option<String>,
    pub trade_date: NaiveDate,
    pub index_code: String,
    pub index_name: Option<String>,
    pub value_type: String,
    pub value: Option<Decimal>,
    pub currency: Option<String>,
    pub market_identifier_code: Option<String>,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct IndexValueFilterParams {
    pub trade_date: Option<NaiveDate>,
    pub index_code: Option<String>,
    pub value_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/index-report/indices",
    tag = "B3 IndexReport (BVBG.087.01)",
    responses(
        (status = 200, description = "List registered indices", body = Vec<IndexItem>)
    )
)]
pub async fn list_indices(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<IndexItem>>, (StatusCode, String)> {
    let rows = sqlx::query("SELECT id, index_code, index_name, currency FROM b3_indices ORDER BY index_code ASC")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| IndexItem {
            id: r.get("id"),
            index_code: r.get("index_code"),
            index_name: r.get("index_name"),
            currency: r.get("currency"),
        })
        .collect();

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/index-report/values",
    tag = "B3 IndexReport (BVBG.087.01)",
    params(PaginationParams, IndexValueFilterParams),
    responses(
        (status = 200, description = "List index values", body = PaginatedResponse<IndexValueItem>)
    )
)]
pub async fn list_index_values(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<IndexValueFilterParams>,
) -> Result<Json<PaginatedResponse<IndexValueItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let idx_code = filters.index_code.as_deref().map(|s| s.trim().to_uppercase());
    let val_type = filters.value_type.as_deref().map(|s| s.trim().to_lowercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_index_report_values v
        WHERE ($1::date IS NULL OR v.trade_date = $1)
          AND ($2::varchar IS NULL OR v.index_code = $2)
          AND ($3::varchar IS NULL OR v.value_type = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&idx_code)
        .bind(&val_type)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            v.id, v.index_id, v.trade_date, v.index_code, v.index_name,
            v.value_type, v.value, v.currency, v.market_identifier_code,
            v.source_file_id, v.source_member_name, v.source_line_number,
            v.data_changed_at, v.field_changed_at, v.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_index_report_values v
        JOIN b3_source_files f ON f.id = v.source_file_id
        WHERE ($1::date IS NULL OR v.trade_date = $1)
          AND ($2::varchar IS NULL OR v.index_code = $2)
          AND ($3::varchar IS NULL OR v.value_type = $3)
        ORDER BY v.trade_date DESC, v.index_code ASC, v.value_type ASC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
        .bind(&idx_code)
        .bind(&val_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| {
            let provenance = Provenance {
                source_file_type: row.get("source_file_type"),
                source_file_id: row.get("source_file_id"),
                source_file_name: row.get("original_file_name"),
                source_member_name: row.get("source_member_name"),
                reference_date: Some(row.get("trade_date")),
                sha256: row.get("sha256"),
                ingested_at: row.get("ingested_at"),
            };

            let internal_metadata = if pagination.include_internal() {
                Some(InternalMetadata {
                    data_changed_at: row.get("data_changed_at"),
                    field_changed_at: row.get("field_changed_at"),
                    resolution_status: Some("resolved".to_string()),
                    last_seen_at: row.get("last_seen_at"),
                    source_line_number: row.get("source_line_number"),
                })
            } else {
                None
            };

            IndexValueItem {
                id: row.get("id"),
                index_id: row.get("index_id"),
                trade_date: row.get("trade_date"),
                index_code: row.get("index_code"),
                index_name: row.get("index_name"),
                value_type: row.get("value_type"),
                value: row.get("value"),
                currency: row.get("currency"),
                market_identifier_code: row.get("market_identifier_code"),
                provenance,
                internal_metadata,
            }
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

pub async fn get_index_values_by_code(
    State(state): State<Arc<AppState>>,
    Path(index_code): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexValueItem>>, (StatusCode, String)> {
    let filters = IndexValueFilterParams {
        trade_date: None,
        index_code: Some(index_code),
        value_type: None,
    };
    list_index_values(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_index_values_by_date(
    State(state): State<Arc<AppState>>,
    Path(trade_date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexValueItem>>, (StatusCode, String)> {
    let filters = IndexValueFilterParams {
        trade_date: Some(trade_date),
        index_code: None,
        value_type: None,
    };
    list_index_values(State(state), Query(pagination), Query(filters)).await
}

