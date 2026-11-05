use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{InternalMetadata, PaginatedResponse, PaginationParams, Provenance};
use crate::api::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TradableSecurityItem {
    pub id: String,
    pub instrument_id: Option<String>,
    pub reference_date: NaiveDate,
    pub security_id: Option<String>,
    pub ticker_symbol: Option<String>,
    pub market_code: Option<String>,
    pub segment_code: Option<String>,
    pub security_category: Option<String>,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
    pub resolution_status: String,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct TradableFilterParams {
    pub reference_date: Option<NaiveDate>,
    pub ticker: Option<String>,
    pub market_code: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/tradable-securities",
    tag = "B3 Tradable Security List",
    params(PaginationParams, TradableFilterParams),
    responses(
        (status = 200, description = "List tradable securities", body = PaginatedResponse<TradableSecurityItem>)
    )
)]
pub async fn list_tradable_securities(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<TradableFilterParams>,
) -> Result<Json<PaginatedResponse<TradableSecurityItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let market = filters.market_code.as_deref().map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_tradable_securities t
        WHERE ($1::date IS NULL OR t.reference_date = $1)
          AND ($2::varchar IS NULL OR t.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR t.market_code = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.reference_date)
        .bind(&ticker)
        .bind(&market)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            t.id, t.instrument_id, t.reference_date, t.security_id, t.ticker_symbol,
            t.market_code, t.segment_code, t.security_category,
            t.trading_start_date, t.trading_end_date, t.resolution_status,
            t.source_file_id, t.source_member_name, t.source_line_number,
            t.data_changed_at, t.field_changed_at, t.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_tradable_securities t
        JOIN b3_source_files f ON f.id = t.source_file_id
        WHERE ($1::date IS NULL OR t.reference_date = $1)
          AND ($2::varchar IS NULL OR t.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR t.market_code = $3)
        ORDER BY t.reference_date DESC, t.ticker_symbol ASC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.reference_date)
        .bind(&ticker)
        .bind(&market)
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
                reference_date: Some(row.get("reference_date")),
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

            TradableSecurityItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                reference_date: row.get("reference_date"),
                security_id: row.get("security_id"),
                ticker_symbol: row.get("ticker_symbol"),
                market_code: row.get("market_code"),
                segment_code: row.get("segment_code"),
                security_category: row.get("security_category"),
                trading_start_date: row.get("trading_start_date"),
                trading_end_date: row.get("trading_end_date"),
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

pub async fn get_tradable_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TradableSecurityItem>>, (StatusCode, String)> {
    let filters = TradableFilterParams {
        reference_date: Some(date),
        ticker: None,
        market_code: None,
    };
    list_tradable_securities(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_tradable_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TradableSecurityItem>>, (StatusCode, String)> {
    let filters = TradableFilterParams {
        reference_date: None,
        ticker: Some(ticker),
        market_code: None,
    };
    list_tradable_securities(State(state), Query(pagination), Query(filters)).await
}

