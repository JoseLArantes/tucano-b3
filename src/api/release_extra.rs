use super::{
    release::{job, AcceptedJob},
    release_error::ApiError,
    AppState,
};
use crate::services::{jobs::JobService, query::*};
use axum::{
    extract::{
        rejection::{JsonRejection, PathRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

/// Retry explícito com nova chave; preserva trabalho e tentativas originais.
#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryRequest {
    /// Nova chave idempotente; não pode ser a chave do trabalho original.
    pub idempotency_key: String,
}
#[utoipa::path(post,path="/api/v1/b3/jobs/{id}/retry",tag="Operação",summary="Submeter retry rastreável de trabalho falho ou cancelado",description="Cria novo job com o mesmo comando, prioridade e limite de tentativas do original.\nPreserva o estado terminal e a auditoria anterior, ligando a nova execução por retry_of.\nReuso da nova chave mantém o mesmo retry; comando/linhagem incompatível retorna conflito.\nExige administração e retorna 202 sem executar ingestão no request.",params(("id"=Uuid,Path,description="UUID terminal original")),request_body=RetryRequest,responses((status=202,description="Novo trabalho aceito",body=AcceptedJob),(status=200,description="Acompanhamento disponível por Location"),(status=400,description="Chave inválida"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Trabalho original inexistente"),(status=409,description="Original não terminal ou conflito idempotente"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn retry(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, PathRejection>,
    payload: Result<Json<RetryRequest>, JsonRejection>,
) -> Result<(StatusCode, HeaderMap, Json<AcceptedJob>), ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    let Json(payload) = payload.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Pedido de retry inválido.",
        )
    })?;
    QueryService::new(state.pool.clone()).job(id).await?;
    let next = JobService::new(state.pool.clone())
        .retry(id, &payload.idempotency_key)
        .await?;
    let status_url = format!("/api/v1/b3/jobs/{next}");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::LOCATION,
        status_url.parse().map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LOCATION_FAILED",
                "Falha no acompanhamento.",
            )
        })?,
    );
    Ok((
        StatusCode::ACCEPTED,
        headers,
        Json(AcceptedJob {
            job_id: next,
            status_url,
        }),
    ))
}
#[utoipa::path(get,path="/api/v1/b3/diagnostics/coverage",tag="COTAHIST",summary="Consultar limites de cobertura e identidade da revisão publicada",description="Diagnóstico agregado da revisão COTAHIST explicitamente publicada e elegível.\nMin/max são observações e não comprovam continuidade de pregões ou universo.\nRejeições, identidade incerta e calendário não demonstrado permanecem visíveis.\nNão dispara aquisição, backfill ou rebuild durante a consulta.",params(DiagnosticsFilters),responses((status=200,description="Cobertura demonstrada",body=DatasetDiagnostics),(status=400,description="Revisão inválida"),(status=401,description="Não autenticado"),(status=403,description="Finalidade não elegível"),(status=404,description="Revisão inexistente"),(status=409,description="Não aplicável à leitura"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma revisão publicada")),security(("bearer_auth"=[])))]
pub async fn diagnostics(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<DiagnosticsFilters>, QueryRejection>,
) -> Result<Json<DatasetDiagnostics>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtro de diagnóstico inválido.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .diagnostics(filters, QueryPurpose::Rest)
            .await?,
    ))
}

#[utoipa::path(get,path="/api/v1/b3/internal/diagnostics/coverage",tag="Operação",summary="Consultar cobertura COTAHIST para uso administrativo interno",description="Retorna limites observados da revisão publicada após validar elegibilidade interna de processamento e retenção bruta. Esta rota requer credencial administrativa e não habilita REST, MCP ou entrega a terceiros.",params(DiagnosticsFilters),responses((status=200,description="Cobertura interna demonstrada",body=DatasetDiagnostics),(status=400,description="Revisão inválida"),(status=401,description="Bearer administrativo ausente ou inválido"),(status=403,description="Acesso administrativo ou elegibilidade interna exigidos"),(status=404,description="Revisão inexistente"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma revisão publicada")),security(("bearer_auth"=[])))]
pub async fn internal_diagnostics(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<DiagnosticsFilters>, QueryRejection>,
) -> Result<Json<DatasetDiagnostics>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtro de diagnóstico inválido.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .diagnostics(filters, QueryPurpose::Internal)
            .await?,
    ))
}

#[utoipa::path(get,path="/api/v1/b3/sync-runs/{id}",tag="Operação",summary="Consultar execução administrativa por identificador durável",description="Projeção tipada do mesmo job compartilhado por API, CLI e worker.\nExpõe fase, tentativas, heartbeat, cancelamento, outputs e código seguro de falha.\nNão transforma consulta de estado em execução de trabalho.\nRequer administração; idempotência de leitura não muda o estado do job.",params(("id"=Uuid,Path,description="UUID do trabalho")),responses((status=200,description="Estado durável",body=JobView),(status=400,description="ID inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Execução inexistente"),(status=409,description="Não aplicável à leitura"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn sync_run(
    state: State<Arc<AppState>>,
    id: Result<Path<Uuid>, PathRejection>,
) -> Result<Json<JobView>, ApiError> {
    job(state, id).await
}
