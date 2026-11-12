//! IBOV API endpoints.
//!
//! All endpoints are under /api/v1/ibov/.

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use utoipa::IntoParams;

use crate::api::health::AppState;
use crate::error::AppResult;
use crate::models::envelope::ApiResponse;
use crate::models::ibov::*;
use crate::models::pagination::{PaginationMeta, PaginationParams};
use crate::services::ibov::IbovService;

/// Parâmetros de consulta para série histórica do IBOV.
#[derive(Debug, Deserialize, IntoParams)]
pub struct IbovHistoryParams {
    /// Data inicial (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final (AAAA-MM-DD)
    pub ate: Option<String>,
    /// Últimos N dias de pregão
    pub dias: Option<u32>,
    /// Ordem: asc ou desc (padrão: desc)
    #[serde(default = "default_order")]
    pub ordem: String,
    /// Número da página
    #[serde(default = "default_page")]
    pub pagina: u32,
    /// Itens por página (padrão: 252)
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

/// Obter série histórica do IBOVESPA
///
/// Retorna a série histórica do índice IBOVESPA com filtros de data e paginação. Inclui variação percentual calculada automaticamente.
#[utoipa::path(
    get,
    path = "/ibov/history",
    tag = "ibov",
    params(
        IbovHistoryParams,
    ),
    responses(
        (status = 200, description = "Série histórica retornada", body = IbovHistoryApiResponse),
        (status = 400, description = "Data inválida", body = ErrorResponse),
        (status = 404, description = "Nenhum dado encontrado", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn history_handler(
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

/// Obter o dado mais recente do IBOVESPA
///
/// Retorna o ponto de dados mais recente do índice IBOVESPA, incluindo variação percentual em relação ao pregão anterior.
#[utoipa::path(
    get,
    path = "/ibov/latest",
    tag = "ibov",
    responses(
        (status = 200, description = "Dado mais recente retornado", body = IbovLatestApiResponse),
        (status = 404, description = "Nenhum dado encontrado", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn latest_handler(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<IbovLatestResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let latest = IbovService::get_latest(&query_service).await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(latest, elapsed)))
}

/// Build IBOV routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ibov/history", get(history_handler))
        .route("/ibov/latest", get(latest_handler))
        .with_state(state)
}
