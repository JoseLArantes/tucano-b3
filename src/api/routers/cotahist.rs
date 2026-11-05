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

pub const COTAHIST_DISCLAIMER: &str = "AVISO: Preços brutos nominais de negociação histórica da B3. Não ajustados retroativamente por proventos em dinheiro (dividendos/JCP), desdobramentos (splits), grupamentos ou bonificações corporativas.";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CotahistQuoteItem {
    pub id: String,
    pub instrument_id: Option<String>,
    pub trade_date: NaiveDate,
    pub bdi_code: Option<String>,
    pub ticker_symbol: Option<String>,
    pub market_type: Option<String>,
    pub issuer_short_name: Option<String>,
    pub paper_specification: Option<String>,
    pub term_days: Option<String>,
    pub currency: Option<String>,
    pub open_price: Option<Decimal>,
    pub high_price: Option<Decimal>,
    pub low_price: Option<Decimal>,
    pub average_price: Option<Decimal>,
    pub last_price: Option<Decimal>,
    pub best_bid_price: Option<Decimal>,
    pub best_ask_price: Option<Decimal>,
    pub trades_count: Option<i32>,
    pub traded_quantity: Option<Decimal>,
    pub traded_volume: Option<Decimal>,
    pub option_exercise_price: Option<Decimal>,
    pub price_correction_indicator: Option<String>,
    pub maturity_date: Option<NaiveDate>,
    pub quotation_factor: Option<i32>,
    pub exercise_price_points: Option<Decimal>,
    pub isin: Option<String>,
    pub distribution_number: Option<String>,
    pub resolution_status: String,
    pub provenance: Provenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct CotahistFilterParams {
    pub trade_date: Option<NaiveDate>,
    pub ticker: Option<String>,
    pub isin: Option<String>,
    pub bdi_code: Option<String>,
    pub market_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes",
    tag = "B3 COTAHIST Historical Quotes",
    params(PaginationParams, CotahistFilterParams),
    responses(
        (status = 200, description = "List COTAHIST quotes (unadjusted nominal prices)", body = PaginatedResponse<CotahistQuoteItem>)
    )
)]
pub async fn list_cotahist_quotes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<CotahistFilterParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let isin = filters.isin.as_deref().map(|s| s.trim().to_uppercase());
    let bdi = filters.bdi_code.as_deref().map(|s| s.trim());
    let mkt = filters.market_type.as_deref().map(|s| s.trim());

    let count_query = r#"
        SELECT count(*)
        FROM b3_cotahist_quotes q
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.isin = $3)
          AND ($4::varchar IS NULL OR q.bdi_code = $4)
          AND ($5::varchar IS NULL OR q.market_type = $5)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(&isin)
        .bind(bdi)
        .bind(mkt)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.bdi_code, q.ticker_symbol,
            q.market_type, q.issuer_short_name, q.paper_specification, q.term_days,
            q.currency, q.open_price, q.high_price, q.low_price, q.average_price,
            q.last_price, q.best_bid_price, q.best_ask_price, q.trades_count,
            q.traded_quantity, q.traded_volume, q.option_exercise_price,
            q.price_correction_indicator, q.maturity_date, q.quotation_factor,
            q.exercise_price_points, q.isin, q.distribution_number, q.resolution_status,
            q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_cotahist_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.isin = $3)
          AND ($4::varchar IS NULL OR q.bdi_code = $4)
          AND ($5::varchar IS NULL OR q.market_type = $5)
        ORDER BY q.trade_date DESC, q.ticker_symbol ASC
        LIMIT $6 OFFSET $7
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(&isin)
        .bind(bdi)
        .bind(mkt)
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

            CotahistQuoteItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                trade_date: row.get("trade_date"),
                bdi_code: row.get("bdi_code"),
                ticker_symbol: row.get("ticker_symbol"),
                market_type: row.get("market_type"),
                issuer_short_name: row.get("issuer_short_name"),
                paper_specification: row.get("paper_specification"),
                term_days: row.get("term_days"),
                currency: row.get("currency"),
                open_price: row.get("open_price"),
                high_price: row.get("high_price"),
                low_price: row.get("low_price"),
                average_price: row.get("average_price"),
                last_price: row.get("last_price"),
                best_bid_price: row.get("best_bid_price"),
                best_ask_price: row.get("best_ask_price"),
                trades_count: row.get("trades_count"),
                traded_quantity: row.get("traded_quantity"),
                traded_volume: row.get("traded_volume"),
                option_exercise_price: row.get("option_exercise_price"),
                price_correction_indicator: row.get("price_correction_indicator"),
                maturity_date: row.get("maturity_date"),
                quotation_factor: row.get("quotation_factor"),
                exercise_price_points: row.get("exercise_price_points"),
                isin: row.get("isin"),
                distribution_number: row.get("distribution_number"),
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
        disclaimer: Some(COTAHIST_DISCLAIMER.to_string()),
    }))
}

pub async fn get_cotahist_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let filters = CotahistFilterParams {
        trade_date: None,
        ticker: Some(ticker),
        isin: None,
        bdi_code: None,
        market_type: None,
    };
    list_cotahist_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_cotahist_by_isin(
    State(state): State<Arc<AppState>>,
    Path(isin): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let filters = CotahistFilterParams {
        trade_date: None,
        ticker: None,
        isin: Some(isin),
        bdi_code: None,
        market_type: None,
    };
    list_cotahist_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_cotahist_by_date(
    State(state): State<Arc<AppState>>,
    Path(trade_date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let filters = CotahistFilterParams {
        trade_date: Some(trade_date),
        ticker: None,
        isin: None,
        bdi_code: None,
        market_type: None,
    };
    list_cotahist_quotes(State(state), Query(pagination), Query(filters)).await
}

pub async fn get_cotahist_by_id(
    State(state): State<Arc<AppState>>,
    Path(quote_id): Path<String>,
) -> Result<Json<CotahistQuoteItem>, (StatusCode, String)> {
    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.bdi_code, q.ticker_symbol,
            q.market_type, q.issuer_short_name, q.paper_specification, q.term_days,
            q.currency, q.open_price, q.high_price, q.low_price, q.average_price,
            q.last_price, q.best_bid_price, q.best_ask_price, q.trades_count,
            q.traded_quantity, q.traded_volume, q.option_exercise_price,
            q.price_correction_indicator, q.maturity_date, q.quotation_factor,
            q.exercise_price_points, q.isin, q.distribution_number, q.resolution_status,
            q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_cotahist_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE q.id = $1
        LIMIT 1
    "#;

    let row = sqlx::query(select_query)
        .bind(&quote_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Cotahist quote not found".to_string(),
        ))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: Some(row.get("trade_date")),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(CotahistQuoteItem {
        id: row.get("id"),
        instrument_id: row.get("instrument_id"),
        trade_date: row.get("trade_date"),
        bdi_code: row.get("bdi_code"),
        ticker_symbol: row.get("ticker_symbol"),
        market_type: row.get("market_type"),
        issuer_short_name: row.get("issuer_short_name"),
        paper_specification: row.get("paper_specification"),
        term_days: row.get("term_days"),
        currency: row.get("currency"),
        open_price: row.get("open_price"),
        high_price: row.get("high_price"),
        low_price: row.get("low_price"),
        average_price: row.get("average_price"),
        last_price: row.get("last_price"),
        best_bid_price: row.get("best_bid_price"),
        best_ask_price: row.get("best_ask_price"),
        trades_count: row.get("trades_count"),
        traded_quantity: row.get("traded_quantity"),
        traded_volume: row.get("traded_volume"),
        option_exercise_price: row.get("option_exercise_price"),
        price_correction_indicator: row.get("price_correction_indicator"),
        maturity_date: row.get("maturity_date"),
        quotation_factor: row.get("quotation_factor"),
        exercise_price_points: row.get("exercise_price_points"),
        isin: row.get("isin"),
        distribution_number: row.get("distribution_number"),
        resolution_status: row.get("resolution_status"),
        provenance,
        internal_metadata: None,
    }))
}
