//! IBOV data models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::pagination::PaginationMeta;

/// A single IBOV historical data point.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IbovHistoryEntry {
    pub data: String,
    pub codigo: String,
    pub nome: Option<String>,
    pub tipo: Option<String>,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub vol_tot: Option<f64>,
    pub num_neg: Option<u64>,
    pub qtd_tot: Option<f64>,
    pub var_pct: Option<f64>,
}

/// Response for GET /api/v1/ibov/history
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IbovHistoryResponse {
    pub codigo: String,
    pub nome: String,
    pub history: Vec<IbovHistoryEntry>,
    pub paginacao: PaginationMeta,
}

/// Response for GET /api/v1/ibov/latest
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IbovLatestResponse {
    pub codigo: String,
    pub nome: String,
    pub data: String,
    pub pre_abr: Option<f64>,
    pub pre_max: Option<f64>,
    pub pre_min: Option<f64>,
    pub pre_med: Option<f64>,
    pub pre_ult: Option<f64>,
    pub vol_tot: Option<f64>,
    pub num_neg: Option<u64>,
    pub qtd_tot: Option<f64>,
    pub var_pct: Option<f64>,
}
