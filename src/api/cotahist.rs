//! COTAHIST API endpoints.
//!
//! All endpoints are under /api/v1/cotahist/.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

use crate::api::health::AppState;
use crate::error::AppResult;
use crate::models::cotahist::*;
use crate::models::envelope::ApiResponse;
use crate::models::pagination::PaginationMeta;
use crate::services::cotahist::CotahistService;

/// Query parameters for GET /tickers
#[derive(Debug, Deserialize)]
pub struct ListTickersParams {
    #[serde(default = "default_page")]
    pub pagina: u32,
    #[serde(default = "default_per_page")]
    pub por_pagina: u32,
    /// Filter by ticker prefix
    pub busca: Option<String>,
    /// Only active tickers (default: true)
    #[serde(default = "default_true")]
    pub ativo: bool,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    50
}
fn default_true() -> bool {
    true
}

/// GET /api/v1/cotahist/tickers
///
/// Lista todos os tickers disponíveis com paginação e busca opcional.
async fn list_tickers_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListTickersParams>,
) -> AppResult<Json<ApiResponse<TickerListResponse>>> {
    let start = Instant::now();

    let pagina = params.pagina.max(1);
    let por_pagina = params.por_pagina.clamp(1, 200);

    let (tickers, total) = CotahistService::list_tickers(
        &state.data_loader.ticker_registry,
        pagina,
        por_pagina,
        params.busca.as_deref(),
        params.ativo,
    )?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(
        TickerListResponse {
            tickers,
            paginacao: PaginationMeta::new(total, pagina, por_pagina),
        },
        elapsed,
    )))
}

/// GET /api/v1/cotahist/tickers/{ticker}
///
/// Retorna metadados de um ticker específico.
async fn get_ticker_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> AppResult<Json<ApiResponse<TickerDetailResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let detail = CotahistService::get_ticker_detail(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(detail, elapsed)))
}

/// Build COTAHIST routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/cotahist/tickers", get(list_tickers_handler))
        .route("/cotahist/tickers/:ticker", get(get_ticker_handler))
        .with_state(state)
}
