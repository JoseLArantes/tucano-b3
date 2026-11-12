//! COTAHIST data models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::pagination::PaginationMeta;

pub use crate::models::cotahist_analytics::*;

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
    pub paginacao: PaginationMeta,
}

/// Response for the quotes endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct QuoteListResponse {
    pub ticker: String,
    pub quotes: Vec<QuoteResponse>,
    pub paginacao: PaginationMeta,
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
    pub paginacao: PaginationMeta,
}

/// Compact daily point for historical charts.
///
/// `date_ms` is the trading date at UTC midnight as Unix epoch milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SeriesPoint {
    /// Data do pregão em milissegundos (Unix epoch, UTC 00:00)
    pub date_ms: i64,
    /// Preço de fechamento (PREULT)
    pub pre_ult: Option<f64>,
    /// Volume total negociado em R$ (VOLTOT)
    pub vol_tot: Option<f64>,
    /// Quantidade total de negócios (TOTNEG)
    pub tot_neg: Option<u32>,
}

/// Response for the compact historical series endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct SeriesResponse {
    pub ticker: String,
    /// Data inicial solicitada (AAAA-MM-DD)
    pub desde: String,
    /// Data final solicitada, se informada (AAAA-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    pub series: Vec<SeriesPoint>,
}

/// Extreme (min or max) closing price for a period.
#[derive(Debug, Serialize, ToSchema)]
pub struct PreUltExtremeResponse {
    pub ticker: String,
    /// Data inicial do período (AAAA-MM-DD)
    pub desde: String,
    /// Data final do período, se informada (AAAA-MM-DD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ate: Option<String>,
    /// Data do pregão em que o extremo ocorreu (AAAA-MM-DD)
    pub data: String,
    /// Preço de fechamento extremo (PREULT)
    pub pre_ult: f64,
}
