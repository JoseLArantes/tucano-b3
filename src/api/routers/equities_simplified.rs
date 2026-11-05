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
pub struct EquitiesSimplifiedQuoteItem {
    pub id: String,
    pub instrument_id: Option<String>,
    pub trade_date: NaiveDate,
    pub security_id: Option<String>,
    pub ticker_symbol: Option<String>,
    pub market_identifier_code: Option<String>,
    pub first_price: Option<Decimal>,
    pub minimum_price: Option<Decimal>,
    pub average_price: Option<Decimal>,
    pub maximum_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub financial_volume_brl: Option<Decimal>,
    pub resolution_status: String,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceComparisonItem {
    pub ticker_symbol: String,
    pub trade_date: NaiveDate,
    pub bvbg086_last_price: Option<Decimal>,
    pub bvbg186_last_price: Option<Decimal>,
    pub diff: Option<Decimal>,
    pub matches: bool,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct EquitiesSimplifiedFilterParams {
    pub trade_date: Option<NaiveDate>,
    pub ticker: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/equities-simplified-price-report/quotes",
    tag = "B3 Simplified Price Report - Equities (BVBG.186.01)",
    params(PaginationParams, EquitiesSimplifiedFilterParams),
    responses(
        (status = 200, description = "List simplified equities quotes", body = PaginatedResponse<EquitiesSimplifiedQuoteItem>)
    )
)]
pub async fn list_equities_simplified_quotes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<EquitiesSimplifiedFilterParams>,
) -> Result<Json<PaginatedResponse<EquitiesSimplifiedQuoteItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_equities_simplified_price_quotes q
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.financial_volume_brl,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_equities_simplified_price_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
        ORDER BY q.trade_date DESC, q.ticker_symbol ASC
        LIMIT $3 OFFSET $4
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
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
                reference_date: Some(row.get("trade_date")),
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

            EquitiesSimplifiedQuoteItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                trade_date: row.get("trade_date"),
                security_id: row.get("security_id"),
                ticker_symbol: row.get("ticker_symbol"),
                market_identifier_code: row.get("market_identifier_code"),
                first_price: row.get("first_price"),
                minimum_price: row.get("minimum_price"),
                average_price: row.get("average_price"),
                maximum_price: row.get("maximum_price"),
                last_price: row.get("last_price"),
                financial_volume_brl: row.get("financial_volume_brl"),
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

pub async fn get_equities_simplified_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<EquitiesSimplifiedQuoteItem>>, (StatusCode, String)> {
    let filters = EquitiesSimplifiedFilterParams {
        trade_date: None,
        ticker: Some(ticker),
    };
    list_equities_simplified_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_equities_simplified_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<EquitiesSimplifiedQuoteItem>>, (StatusCode, String)> {
    let filters = EquitiesSimplifiedFilterParams {
        trade_date: Some(date),
        ticker: None,
    };
    list_equities_simplified_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_equities_simplified_by_id(
    State(state): State<Arc<AppState>>,
    Path(quote_id): Path<String>,
) -> Result<Json<EquitiesSimplifiedQuoteItem>, (StatusCode, String)> {
    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.financial_volume_brl,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_equities_simplified_price_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE q.id = $1
        LIMIT 1
    "#;

    let row = sqlx::query(select_query)
        .bind(&quote_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Simplified quote not found".to_string()))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: Some(row.get("trade_date")),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(EquitiesSimplifiedQuoteItem {
        id: row.get("id"),
        instrument_id: row.get("instrument_id"),
        trade_date: row.get("trade_date"),
        security_id: row.get("security_id"),
        ticker_symbol: row.get("ticker_symbol"),
        market_identifier_code: row.get("market_identifier_code"),
        first_price: row.get("first_price"),
        minimum_price: row.get("minimum_price"),
        average_price: row.get("average_price"),
        maximum_price: row.get("maximum_price"),
        last_price: row.get("last_price"),
        financial_volume_brl: row.get("financial_volume_brl"),
        resolution_status: row.get("resolution_status"),
        provenance,
        internal_metadata: None,
    }))
}

pub async fn compare_price_reports(
    State(state): State<Arc<AppState>>,
    Path(trade_date): Path<NaiveDate>,
) -> Result<Json<Vec<PriceComparisonItem>>, (StatusCode, String)> {
    let query = r#"
        SELECT 
            COALESCE(p.ticker_symbol, s.ticker_symbol) as ticker,
            p.trade_date,
            p.last_price as p_last,
            s.last_price as s_last
        FROM b3_price_report_quotes p
        FULL OUTER JOIN b3_equities_simplified_price_quotes s 
            ON s.trade_date = p.trade_date AND s.ticker_symbol = p.ticker_symbol
        WHERE (p.trade_date = $1 OR s.trade_date = $1)
        ORDER BY ticker ASC
    "#;

    let rows = sqlx::query(query)
        .bind(trade_date)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| {
            let p_last: Option<Decimal> = r.get("p_last");
            let s_last: Option<Decimal> = r.get("s_last");
            let diff = match (p_last, s_last) {
                (Some(p), Some(s)) => Some(p - s),
                _ => None,
            };
            let matches = match diff {
                Some(d) => d.is_zero(),
                None => false,
            };

            PriceComparisonItem {
                ticker_symbol: r.get("ticker"),
                trade_date,
                bvbg086_last_price: p_last,
                bvbg186_last_price: s_last,
                diff,
                matches,
            }
        })
        .collect();

    Ok(Json(items))
}

