//! Public index history and manual B3 CSV ingestion endpoints.

use crate::api::health::AppState;
use crate::error::AppResult;
#[allow(unused_imports)]
use crate::error::ErrorResponse;
#[allow(unused_imports)]
use crate::ingest::index_metrics::IngestJob;
use crate::ingest::index_metrics::{IndexCode, ManualIngestRequest};
use crate::models::envelope::ApiResponse;
#[allow(unused_imports)]
use crate::models::envelope::{IbovHistoryApiResponse, IbovLatestApiResponse};
use crate::models::ibov::{IbovHistoryResponse, IbovLatestResponse};
use crate::models::pagination::{PaginationMeta, PaginationParams};
use crate::services::ibov::IbovService;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use utoipa::IntoParams;
use utoipa::ToSchema;

#[derive(Debug, Deserialize, IntoParams)]
pub struct HistoryParams {
    /// Data inicial inclusiva no formato AAAA-MM-DD.
    pub desde: Option<String>,
    /// Data final inclusiva no formato AAAA-MM-DD.
    pub ate: Option<String>,
    /// Restringe a consulta aos últimos N pregões disponíveis.
    pub dias: Option<u32>,
    /// Ordenação cronológica, asc ou desc. O padrão é desc.
    #[serde(default = "default_order")]
    pub ordem: String,
    /// Página iniciada em 1.
    #[serde(default = "default_page")]
    pub pagina: u32,
    /// Quantidade de itens por página, padrão 252 e máximo 1000.
    #[serde(default = "default_per_page")]
    pub por_pagina: u32,
}
fn default_order() -> String {
    "desc".into()
}
fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    252
}

#[derive(serde::Serialize, ToSchema)]
pub struct IndexIngestStatus {
    /// Horário da última partição publicada com sucesso.
    pub last_successful_ingestion: Option<String>,
    /// Última data de negociação disponível na tabela de índices.
    pub latest_trading_date: Option<String>,
}

#[derive(serde::Serialize, ToSchema)]
pub struct IngestJobResponse {
    /// Estado persistido da execução.
    pub data: IngestJob,
}

#[derive(serde::Serialize, ToSchema)]
pub struct IngestJobReceipt {
    /// Identificador para consultar a execução.
    pub id: String,
    /// Estado inicial da fila.
    pub status: String,
}

async fn history_impl(
    state: Arc<AppState>,
    index: IndexCode,
    name: &'static str,
    params: HistoryParams,
) -> AppResult<Json<ApiResponse<IbovHistoryResponse>>> {
    let started = Instant::now();
    let query = state.data_loader.get_query_service().await;
    let pagination = PaginationParams {
        pagina: params.pagina,
        por_pagina: params.por_pagina,
    }
    .validated();
    let (rows, total) = IbovService::get_index_history(
        &query,
        index.as_str(),
        params.desde.as_deref(),
        params.ate.as_deref(),
        params.dias,
        &params.ordem,
        &pagination,
    )
    .await?;
    Ok(Json(ApiResponse::new(
        IbovHistoryResponse {
            codigo: index.as_str().to_string(),
            nome: name.to_string(),
            history: rows,
            paginacao: PaginationMeta::new(total, pagination.pagina, pagination.por_pagina),
        },
        started.elapsed().as_secs_f64() * 1000.0,
    )))
}

async fn latest_impl(
    state: Arc<AppState>,
    index: IndexCode,
) -> AppResult<Json<ApiResponse<IbovLatestResponse>>> {
    let started = Instant::now();
    let query = state.data_loader.get_query_service().await;
    let latest = IbovService::get_index_latest(&query, index.as_str()).await?;
    Ok(Json(ApiResponse::new(
        latest,
        started.elapsed().as_secs_f64() * 1000.0,
    )))
}

#[utoipa::path(get,path="/metrics/ibov/history",tag="índices",params(HistoryParams),responses((status=200,description="Histórico diário do IBOV em pontos",body=IbovHistoryApiResponse),(status=400,description="Parâmetros inválidos",body=ErrorResponse),(status=404,description="Sem dados para o intervalo",body=ErrorResponse)),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn ibov_history(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HistoryParams>,
) -> AppResult<Json<ApiResponse<IbovHistoryResponse>>> {
    history_impl(state, IndexCode::Ibov, "IBOVESPA", params).await
}
#[utoipa::path(get,path="/metrics/idiv/history",tag="índices",params(HistoryParams),responses((status=200,description="Histórico diário do IDIV em pontos",body=IbovHistoryApiResponse),(status=400,description="Parâmetros inválidos",body=ErrorResponse),(status=404,description="Sem dados para o intervalo",body=ErrorResponse)),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn idiv_history(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HistoryParams>,
) -> AppResult<Json<ApiResponse<IbovHistoryResponse>>> {
    history_impl(state, IndexCode::Idiv, "IDIV", params).await
}
#[utoipa::path(get,path="/metrics/ibov/latest",tag="índices",responses((status=200,description="Fechamento mais recente do IBOV em pontos",body=IbovLatestApiResponse),(status=404,description="Sem dados",body=ErrorResponse)),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn ibov_latest(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<IbovLatestResponse>>> {
    latest_impl(state, IndexCode::Ibov).await
}
#[utoipa::path(get,path="/metrics/ifix/latest",tag="índices",responses((status=200,description="Fechamento mais recente do IFIX em pontos",body=IbovLatestApiResponse),(status=404,description="Sem dados",body=ErrorResponse)),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn ifix_latest(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<IbovLatestResponse>>> {
    latest_impl(state, IndexCode::Ifix).await
}

#[utoipa::path(post,path="/ingest/indices",tag="ingestão",request_body=ManualIngestRequest,responses((status=202,description="Ingestão enfileirada",body=IngestJobReceipt),(status=400,description="Parâmetros inválidos",body=ErrorResponse),(status=409,description="Já existe uma execução de ingestão")),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn start_ingest(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ManualIngestRequest>,
) -> Result<(StatusCode, Json<IngestJobReceipt>), (StatusCode, Json<serde_json::Value>)> {
    if !state.auth_enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(
                json!({"error":{"code":"INGEST_AUTH_NOT_CONFIGURED","message":"Configure uma credencial de API antes de usar a ingestão de índices."}}),
            ),
        ));
    }
    let running = state.index_ingestor.has_active_job().unwrap_or(true);
    if running {
        return Err((
            StatusCode::CONFLICT,
            Json(
                json!({"error":{"code":"INGEST_RUNNING","message":"Já existe uma ingestão em andamento."}}),
            ),
        ));
    }
    match state.index_ingestor.create_job(request) {
        Ok(id) => Ok((
            StatusCode::ACCEPTED,
            Json(IngestJobReceipt {
                id,
                status: "queued".into(),
            }),
        )),
        Err(e) => {
            let conflict = e.to_string().contains("already active");
            Err((
                if conflict {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::BAD_REQUEST
                },
                Json(
                    json!({"error":{"code":if conflict {"INGEST_RUNNING"}else{"INVALID_INGEST_REQUEST"},"message":e.to_string()}}),
                ),
            ))
        }
    }
}

#[utoipa::path(get,path="/ingest/indices/{id}",tag="ingestão",params(("id"=String,Path,description="Identificador da execução")),responses((status=200,description="Status da ingestão",body=IngestJobResponse),(status=404,description="Execução não encontrada",body=ErrorResponse)),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn ingest_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<IngestJobResponse>, (StatusCode, Json<serde_json::Value>)> {
    if !state.auth_enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(
                json!({"error":{"code":"INGEST_AUTH_NOT_CONFIGURED","message":"Configure uma credencial de API antes de consultar a ingestão de índices."}}),
            ),
        ));
    }
    match state.index_ingestor.job(&id) {
        Ok(Some(data)) => Ok(Json(IngestJobResponse { data })),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error":{"code":"JOB_NOT_FOUND","message":"Execução não encontrada."}})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":{"code":"INTERNAL_ERROR","message":e.to_string()}})),
        )),
    }
}

#[utoipa::path(get,path="/ingest/indices/status",tag="ingestão",responses((status=200,description="Última ingestão e data de negociação",body=IndexIngestStatus)),security(("BearerAuth"=[]),("ApiKeyAuth"=[])))]
pub(crate) async fn ingestion_overview(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IndexIngestStatus>, (StatusCode, Json<serde_json::Value>)> {
    if !state.auth_enabled {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(
                json!({"error":{"code":"INGEST_AUTH_NOT_CONFIGURED","message":"Configure uma credencial de API antes de consultar a ingestão de índices."}}),
            ),
        ));
    }
    let last = state
        .index_ingestor
        .last_successful_ingestion()
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":{"message":e.to_string()}})),
            )
        })?;
    let query = state.data_loader.get_query_service().await;
    let latest = if query.has_table("index_metrics") {
        let rows = query
            .execute_sql("SELECT MAX(date) latest FROM index_metrics")
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error":{"message":e.to_string()}})),
                )
            })?;
        rows.first()
            .and_then(|b| crate::services::ibov::IbovService::date_value(b, 0, "latest"))
    } else {
        None
    };
    Ok(Json(IndexIngestStatus {
        last_successful_ingestion: last,
        latest_trading_date: latest,
    }))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/metrics/ibov/history", get(ibov_history))
        .route("/metrics/ibov/latest", get(ibov_latest))
        .route("/metrics/idiv/history", get(idiv_history))
        .route("/metrics/ifix/latest", get(ifix_latest))
        .route("/ingest/indices", post(start_ingest))
        .route("/ingest/indices/status", get(ingestion_overview))
        .route("/ingest/indices/:id", get(ingest_status))
        .with_state(state)
}
