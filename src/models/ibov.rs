//! IBOV data models.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::pagination::PaginationMeta;

/// A single IBOV historical data point.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IbovHistoryEntry {
    /// Data do pregão em ISO 8601.
    pub data: String,
    /// Código do índice.
    pub codigo: String,
    /// Nome do índice.
    pub nome: Option<String>,
    /// Tipo publicado pela origem, se disponível.
    pub tipo: Option<String>,
    /// Abertura do índice em pontos, nula para dados CSV.
    pub pre_abr: Option<f64>,
    /// Máxima do índice em pontos, nula para dados CSV.
    pub pre_max: Option<f64>,
    /// Mínima do índice em pontos, nula para dados CSV.
    pub pre_min: Option<f64>,
    /// Valor médio em pontos, nulo para dados CSV.
    pub pre_med: Option<f64>,
    /// Fechamento oficial em pontos.
    pub pre_ult: Option<f64>,
    /// Volume, nulo para dados CSV.
    pub vol_tot: Option<f64>,
    /// Número de negócios, nulo para dados CSV.
    pub num_neg: Option<u64>,
    /// Quantidade negociada, nula para dados CSV.
    pub qtd_tot: Option<f64>,
    /// Variação percentual em relação ao pregão disponível anterior.
    pub var_pct: Option<f64>,
}

/// Response for GET /api/v1/ibov/history
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IbovHistoryResponse {
    /// Código do índice.
    pub codigo: String,
    /// Nome do índice.
    pub nome: String,
    /// Observações do intervalo solicitado.
    pub history: Vec<IbovHistoryEntry>,
    /// Metadados de paginação.
    pub paginacao: PaginationMeta,
}

/// Response for GET /api/v1/ibov/latest
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IbovLatestResponse {
    /// Código do índice.
    pub codigo: String,
    /// Nome do índice.
    pub nome: String,
    /// Data do pregão mais recente em ISO 8601.
    pub data: String,
    /// Abertura em pontos, nula para dados CSV.
    pub pre_abr: Option<f64>,
    /// Máxima em pontos, nula para dados CSV.
    pub pre_max: Option<f64>,
    /// Mínima em pontos, nula para dados CSV.
    pub pre_min: Option<f64>,
    /// Valor médio em pontos, nulo para dados CSV.
    pub pre_med: Option<f64>,
    /// Fechamento oficial em pontos.
    pub pre_ult: Option<f64>,
    /// Volume, nulo para dados CSV.
    pub vol_tot: Option<f64>,
    /// Número de negócios, nulo para dados CSV.
    pub num_neg: Option<u64>,
    /// Quantidade negociada, nula para dados CSV.
    pub qtd_tot: Option<f64>,
    /// Variação percentual em relação ao pregão disponível anterior.
    pub var_pct: Option<f64>,
}
