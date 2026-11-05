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
pub struct PriceReportQuoteItem {
    pub id: String,
    pub instrument_id: Option<String>,
    pub trade_date: NaiveDate,
    pub security_id: Option<String>,
    pub ticker_symbol: Option<String>,
    pub market_identifier_code: Option<String>,
    pub days_to_settlement: Option<i32>,
    pub trade_quantity: Option<Decimal>,
    pub financial_volume_brl: Option<Decimal>,
    pub financial_volume_usd: Option<Decimal>,
    pub open_interest: Option<Decimal>,
    pub best_bid_price: Option<Decimal>,
    pub best_ask_price: Option<Decimal>,
    pub first_price: Option<Decimal>,
    pub minimum_price: Option<Decimal>,
    pub average_price: Option<Decimal>,
    pub maximum_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub regular_transactions_quantity: Option<i32>,
    pub resolution_status: String,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PriceReportFilterParams {
    pub trade_date: Option<NaiveDate>,
    pub ticker: Option<String>,
    pub security_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/price-report/quotes",
    tag = "B3 PriceReport (BVBG.086.01)",
    params(PaginationParams, PriceReportFilterParams),
    responses(
        (status = 200, description = "List price report quotes", body = PaginatedResponse<PriceReportQuoteItem>)
    )
)]
pub async fn list_price_report_quotes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<PriceReportFilterParams>,
) -> Result<Json<PaginatedResponse<PriceReportQuoteItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let sec_id = filters.security_id.as_deref().map(|s| s.trim());

    let count_query = r#"
        SELECT count(*)
        FROM b3_price_report_quotes q
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.security_id = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(&sec_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.days_to_settlement, q.trade_quantity,
            q.financial_volume_brl, q.financial_volume_usd, q.open_interest,
            q.best_bid_price, q.best_ask_price, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.regular_transactions_quantity,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_price_report_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.security_id = $3)
        ORDER BY q.trade_date DESC, q.ticker_symbol ASC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(&sec_id)
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

            PriceReportQuoteItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                trade_date: row.get("trade_date"),
                security_id: row.get("security_id"),
                ticker_symbol: row.get("ticker_symbol"),
                market_identifier_code: row.get("market_identifier_code"),
                days_to_settlement: row.get("days_to_settlement"),
                trade_quantity: row.get("trade_quantity"),
                financial_volume_brl: row.get("financial_volume_brl"),
                financial_volume_usd: row.get("financial_volume_usd"),
                open_interest: row.get("open_interest"),
                best_bid_price: row.get("best_bid_price"),
                best_ask_price: row.get("best_ask_price"),
                first_price: row.get("first_price"),
                minimum_price: row.get("minimum_price"),
                average_price: row.get("average_price"),
                maximum_price: row.get("maximum_price"),
                last_price: row.get("last_price"),
                regular_transactions_quantity: row.get("regular_transactions_quantity"),
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

pub async fn get_price_report_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PriceReportQuoteItem>>, (StatusCode, String)> {
    let filters = PriceReportFilterParams {
        trade_date: None,
        ticker: Some(ticker),
        security_id: None,
    };
    list_price_report_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_price_report_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PriceReportQuoteItem>>, (StatusCode, String)> {
    let filters = PriceReportFilterParams {
        trade_date: Some(date),
        ticker: None,
        security_id: None,
    };
    list_price_report_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_price_report_by_id(
    State(state): State<Arc<AppState>>,
    Path(quote_id): Path<String>,
) -> Result<Json<PriceReportQuoteItem>, (StatusCode, String)> {
    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.days_to_settlement, q.trade_quantity,
            q.financial_volume_brl, q.financial_volume_usd, q.open_interest,
            q.best_bid_price, q.best_ask_price, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.regular_transactions_quantity,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_price_report_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE q.id = $1
        LIMIT 1
    "#;

    let row = sqlx::query(select_query)
        .bind(&quote_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Price report quote not found".to_string()))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: Some(row.get("trade_date")),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(PriceReportQuoteItem {
        id: row.get("id"),
        instrument_id: row.get("instrument_id"),
        trade_date: row.get("trade_date"),
        security_id: row.get("security_id"),
        ticker_symbol: row.get("ticker_symbol"),
        market_identifier_code: row.get("market_identifier_code"),
        days_to_settlement: row.get("days_to_settlement"),
        trade_quantity: row.get("trade_quantity"),
        financial_volume_brl: row.get("financial_volume_brl"),
        financial_volume_usd: row.get("financial_volume_usd"),
        open_interest: row.get("open_interest"),
        best_bid_price: row.get("best_bid_price"),
        best_ask_price: row.get("best_ask_price"),
        first_price: row.get("first_price"),
        minimum_price: row.get("minimum_price"),
        average_price: row.get("average_price"),
        maximum_price: row.get("maximum_price"),
        last_price: row.get("last_price"),
        regular_transactions_quantity: row.get("regular_transactions_quantity"),
        resolution_status: row.get("resolution_status"),
        provenance,
        internal_metadata: None,
    }))
}

