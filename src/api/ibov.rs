//! IBOV API endpoints.
//!
//! All endpoints are under /api/v1/ibov/.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

use crate::api::health::AppState;
use crate::error::AppResult;
use crate::models::envelope::ApiResponse;
use crate::models::ibov::*;
use crate::models::pagination::{PaginationMeta, PaginationParams};
use crate::services::ibov::IbovService;

/// Query parameters for GET /ibov/history
#[derive(Debug, Deserialize)]
pub struct IbovHistoryParams {
    /// Start date (YYYY-MM-DD)
    pub desde: Option<String>,
    /// End date (YYYY-MM-DD)
    pub ate: Option<String>,
    /// Last N trading days
    pub dias: Option<u32>,
    /// Sort order: asc or desc (default: desc)
    #[serde(default = "default_order")]
    pub ordem: String,
    #[serde(default = "default_page")]
    pub pagina: u32,
    #[serde(default = "default_per_page")]
    pub por_pagina: u32,
}

fn default_order() -> String {
    "desc".to_string()
}
fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    252
}

/// GET /api/v1/ibov/history
///
/// Retorna a série histórica do IBOVESPA com filtros e paginação.
async fn history_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IbovHistoryParams>,
) -> AppResult<Json<ApiResponse<IbovHistoryResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let pagination = PaginationParams {
        pagina: params.pagina,
        por_pagina: params.por_pagina,
    }
    .validated();

    let (history, total) = IbovService::get_history(
        &query_service,
        params.desde.as_deref(),
        params.ate.as_deref(),
        params.dias,
        &params.ordem,
        &pagination,
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    // Extract codigo and nome from first entry (or defaults)
    let codigo = history
        .first()
        .map(|h| h.codigo.clone())
        .unwrap_or_else(|| "IBOV".to_string());
    let nome = history
        .first()
        .and_then(|h| h.nome.clone())
        .unwrap_or_else(|| "IBOVESPA".to_string());

    Ok(Json(ApiResponse::new(
        IbovHistoryResponse {
            codigo,
            nome,
            history,
            paginacao: PaginationMeta::new(total, pagination.pagina, pagination.por_pagina),
        },
        elapsed,
    )))
}

/// Build IBOV routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ibov/history", get(history_handler))
        .with_state(state)
}
