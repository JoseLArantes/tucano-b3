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
pub struct IndexCompositionItem {
    pub id: String,
    pub index_id: Option<String>,
    pub instrument_id: Option<String>,
    pub composition_date: NaiveDate,
    pub index_code: String,
    pub ticker_symbol: Option<String>,
    pub isin: Option<String>,
    pub asset_name: Option<String>,
    pub theoretical_quantity: Option<Decimal>,
    pub portfolio_weight: Option<Decimal>,
    pub participation_percentage: Option<Decimal>,
    pub resolution_status: String,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct IndexCompositionFilterParams {
    pub composition_date: Option<NaiveDate>,
    pub index_code: Option<String>,
    pub ticker: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/index-composition/portfolios",
    tag = "B3 Index Portfolio Composition",
    params(PaginationParams, IndexCompositionFilterParams),
    responses(
        (status = 200, description = "List index portfolio components", body = PaginatedResponse<IndexCompositionItem>)
    )
)]
pub async fn list_index_compositions(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<IndexCompositionFilterParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let idx_code = filters.index_code.as_deref().map(|s| s.trim().to_uppercase());
    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_index_portfolio_compositions c
        WHERE ($1::date IS NULL OR c.composition_date = $1)
          AND ($2::varchar IS NULL OR c.index_code = $2)
          AND ($3::varchar IS NULL OR c.ticker_symbol = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.composition_date)
        .bind(&idx_code)
        .bind(&ticker)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            c.id, c.index_id, c.instrument_id, c.composition_date, c.index_code,
            c.ticker_symbol, c.isin, c.asset_name, c.theoretical_quantity,
            c.portfolio_weight, c.participation_percentage, c.resolution_status,
            c.source_file_id, c.source_member_name, c.source_line_number,
            c.data_changed_at, c.field_changed_at, c.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_index_portfolio_compositions c
        JOIN b3_source_files f ON f.id = c.source_file_id
        WHERE ($1::date IS NULL OR c.composition_date = $1)
          AND ($2::varchar IS NULL OR c.index_code = $2)
          AND ($3::varchar IS NULL OR c.ticker_symbol = $3)
        ORDER BY c.composition_date DESC, c.index_code ASC, c.portfolio_weight DESC NULLS LAST
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.composition_date)
        .bind(&idx_code)
        .bind(&ticker)
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
                reference_date: Some(row.get("composition_date")),
                sha256: row.get("sha256"),
                ingested_at: row.get("ingested_at"),
            };

            let internal_metadata = if pagination.include_internal() {
                Some(InternalMetadata {
                    data_changed_at: row.get("data_changed_at"),
                    field_changed_at: row.get("field_changed_at"),
                    resolution_status: Some(row.get("resolution_status")),
                    last_seen_at: row.get("last_seen_at"),
                    source_line_number: row.get("source_line_number"),
                })
            } else {
                None
            };

            IndexCompositionItem {
                id: row.get("id"),
                index_id: row.get("index_id"),
                instrument_id: row.get("instrument_id"),
                composition_date: row.get("composition_date"),
                index_code: row.get("index_code"),
                ticker_symbol: row.get("ticker_symbol"),
                isin: row.get("isin"),
                asset_name: row.get("asset_name"),
                theoretical_quantity: row.get("theoretical_quantity"),
                portfolio_weight: row.get("portfolio_weight"),
                participation_percentage: row.get("participation_percentage"),
                resolution_status: row.get("resolution_status"),
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

pub async fn get_index_composition_by_index(
    State(state): State<Arc<AppState>>,
    Path(index_code): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let filters = IndexCompositionFilterParams {
        composition_date: None,
        index_code: Some(index_code),
        ticker: None,
    };
    list_index_compositions(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_index_composition_by_index_and_date(
    State(state): State<Arc<AppState>>,
    Path((index_code, date)): Path<(String, NaiveDate)>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let filters = IndexCompositionFilterParams {
        composition_date: Some(date),
        index_code: Some(index_code),
        ticker: None,
    };
    list_index_compositions(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_index_composition_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let filters = IndexCompositionFilterParams {
        composition_date: None,
        index_code: None,
        ticker: Some(ticker),
    };
    list_index_compositions(State(state), Query(pagination), Query(filters)).await
}

