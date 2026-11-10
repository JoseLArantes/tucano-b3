//! COTAHIST data models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single COTAHIST quote record.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub ticker: String,
    pub data: String,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub pre_ofe_compra: Option<f64>,
    pub pre_ofe_venda: Option<f64>,
    pub tot_neg: Option<u32>,
    pub quat_tot: Option<u64>,
    pub vol_tot: Option<f64>,
    pub var_pct: Option<f64>,
    pub var_abs: Option<f64>,
}

/// Ticker metadata from the ledger.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerResponse {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_active: bool,
}

/// Extended ticker info with latest quote data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TickerDetailResponse {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_active: bool,
    pub total_trading_days: Option<u32>,
    pub latest_price: Option<f64>,
    pub latest_date: Option<String>,
}

/// Response for the ticker list endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct TickerListResponse {
    pub tickers: Vec<TickerResponse>,
    pub paginacao: crate::models::pagination::PaginationMeta,
}

/// Response for the quotes endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct QuoteListResponse {
    pub ticker: String,
    pub quotes: Vec<QuoteResponse>,
    pub paginacao: crate::models::pagination::PaginationMeta,
}

/// Response for the latest quote endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct LatestQuoteResponse {
    pub ticker: String,
    pub data: String,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub pre_ofe_compra: Option<f64>,
    pub pre_ofe_venda: Option<f64>,
    pub tot_neg: Option<u32>,
    pub quat_tot: Option<u64>,
    pub vol_tot: Option<f64>,
    pub var_pct: Option<f64>,
    pub var_abs: Option<f64>,
}

/// Response for the date quotes endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct DateQuotesResponse {
    pub date: String,
    pub quotes: Vec<QuoteResponse>,
    pub paginacao: crate::models::pagination::PaginationMeta,
}
