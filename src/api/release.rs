use super::{release_error::ApiError, AppState};
use crate::services::query::{
    InstrumentFilters, InstrumentIdentifierPage, InstrumentPage, ListFilters,
};
use crate::services::{
    jobs::{JobCommand, JobPriority, JobService},
    query::{JobView, QueryError, QueryPurpose, QueryService, QuoteFilters, QuotePage},
};
use axum::{
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(get,path="/api/v1/b3/instruments",tag="Cadastro",summary="Buscar evidências cadastrais oficiais publicadas por fotografia",description="Consulta BVBG.028/Tradable Security List em fotografias explicitamente publicadas.\nPreserva namespace, mercado, data de referência e proveniência; não inventa início de validade.\nFamílias distintas permanecem distinguíveis; ausência de código forte não fabrica security_id.\nFiltros e paginação não disparam aquisição, resolução pesada ou publicação.",params(InstrumentFilters),responses((status=200,description="Página de evidências",body=InstrumentPage),(status=400,description="Filtro inválido"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Fotografia inexistente"),(status=409,description="Conflito de identidade não escolhido arbitrariamente"),(status=500,description="Falha interna"),(status=503,description="Nenhuma fotografia publicada")),security(("bearer_auth"=[])))]
pub async fn instruments(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<InstrumentFilters>, QueryRejection>,
) -> Result<Json<InstrumentPage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtros cadastrais inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instruments(filters, QueryPurpose::Rest)
            .await?,
    ))
}

/// Query all currently published observations for one canonical instrument.
#[utoipa::path(get,path="/api/v1/b3/instruments/{id}",tag="Cadastro",summary="Consultar evidências publicadas para um instrumento canônico",description="Retorna todas as evidências cadastrais ativas para o UUID canônico informado, sem escolher arbitrariamente uma fotografia ou família.\nCada item preserva snapshot, referência, qualificador e proveniência; datas de fotografia não são convertidas em validade histórica.\nFiltros cadastrais podem reduzir a página, mas o ID da rota prevalece. A consulta não dispara aquisição, parsing ou rebuild.",params(("id"=Uuid,Path,description="UUID estável do instrumento"),InstrumentFilters),responses((status=200,description="Evidências publicadas paginadas",body=InstrumentPage),(status=400,description="ID ou filtros inválidos"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Instrumento sem evidência publicada"),(status=409,description="Conflito de identidade"),(status=500,description="Falha de dependência"),(status=503,description="Registro cadastral indisponível")),security(("bearer_auth"=[])))]
pub async fn instrument(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    filters: Result<Query<InstrumentFilters>, QueryRejection>,
) -> Result<Json<InstrumentPage>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtros cadastrais inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instrument(id, filters, QueryPurpose::Rest)
            .await?,
    ))
}

/// Query temporal identifier evidence from all retained published snapshots.
#[utoipa::path(get,path="/api/v1/b3/instruments/{id}/identifiers",tag="Cadastro",summary="Consultar identificadores qualificados e janelas comprovadas",description="Lista ticker, ISIN e código de segurança com qualificadores de mercado, validade comprovada e fotografia de origem.\nA data de referência da fotografia não é promovida a intervalo histórico presumido.\nA paginação é determinística e cada item aponta ao artefato, member e registro disponíveis.\nA consulta somente leitura bloqueia fontes sem elegibilidade para REST e terceiros.",params(("id"=Uuid,Path,description="UUID estável do instrumento"),ListFilters),responses((status=200,description="Página de evidências temporais",body=InstrumentIdentifierPage),(status=400,description="ID ou paginação inválidos"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Instrumento sem identificadores publicados"),(status=409,description="Conflito não aplicável à consulta"),(status=500,description="Falha de dependência")),security(("bearer_auth"=[])))]
pub async fn instrument_identifiers(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    filters: Result<Query<ListFilters>, QueryRejection>,
) -> Result<Json<InstrumentIdentifierPage>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Paginação inválida.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instrument_identifiers(id, filters, QueryPurpose::Rest)
            .await?,
    ))
}

/// Query one registry family without falling back to BVBG.028.
#[utoipa::path(get,path="/api/v1/b3/tradable-securities",tag="Cadastro",summary="Consultar evidências publicadas da Tradable Security List",description="Aplica exclusivamente a família Tradable Security List por meio do serviço cadastral compartilhado.\nPreserva fotografia e proveniência, e não fabrica indicadores ausentes da projeção tipada.\nNão consulta BVBG.028 como fallback e bloqueia fontes/finalidades sem elegibilidade.\nFiltros e paginação não iniciam aquisição ou parsing.",params(InstrumentFilters),responses((status=200,description="Página da Tradable Security List",body=InstrumentPage),(status=400,description="Filtro inválido ou família conflitante"),(status=401,description="Não autenticado"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Snapshot inexistente"),(status=409,description="Conflito de seleção"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma fotografia publicada")),security(("bearer_auth"=[])))]
pub async fn tradable_securities(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<InstrumentFilters>, QueryRejection>,
) -> Result<Json<InstrumentPage>, ApiError> {
    let Query(mut filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Filtros cadastrais inválidos.",
        )
    })?;
    let family = crate::db::enums::SourceFileType::TradableSecurityList;
    if filters
        .source_file_type
        .is_some_and(|value| value != family)
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Esta rota aceita somente a família Tradable Security List.",
        ));
    }
    filters.source_file_type = Some(family);
    Ok(Json(
        QueryService::new(state.pool.clone())
            .instruments(filters, QueryPurpose::Rest)
            .await?,
    ))
}

impl From<QueryError> for ApiError {
    fn from(error: QueryError) -> Self {
        match error {
            QueryError::Invalid(_) => Self::new(
                StatusCode::BAD_REQUEST,
                "INVALID_FILTER",
                "Filtro, intervalo ou paginação inválidos.",
            ),
            QueryError::Unavailable => Self::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "REVISION_UNAVAILABLE",
                "Nenhuma revisão publicável está disponível.",
            ),
            QueryError::NotFound => Self::new(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Recurso individual não encontrado.",
            ),
            QueryError::Forbidden => Self::new(
                StatusCode::FORBIDDEN,
                "SOURCE_NOT_ELIGIBLE",
                "A fonte não está habilitada para esta finalidade.",
            ),
            QueryError::Ambiguous => Self::new(
                StatusCode::CONFLICT,
                "AMBIGUOUS_SERIES",
                "Selecione mercado, distribuição e prazo para obter uma série inequívoca.",
            ),
            QueryError::Database(_) => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_FAILED",
                "Falha na dependência de consulta.",
            ),
        }
    }
}

/// Submissão explícita de trabalho administrativo; não executa parsing no request.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitJob {
    /// Chave idempotente; reuso com comando diferente retorna 409.
    pub idempotency_key: String,
    /// Operação fechada sobre fontes oficiais, arquivos e revisões.
    pub command: JobCommand,
    /// Classe de prioridade; manutenção dispõe de capacidade reservada.
    pub priority: JobPriority,
    /// Limite de tentativas, padrão 3; intervalo 1–100.
    #[schema(default = 3, minimum = 1, maximum = 100)]
    pub max_attempts: Option<i32>,
}
/// Referência de acompanhamento; 202 não significa ingestão ou publicação concluída.
#[derive(Debug, Serialize, ToSchema)]
pub struct AcceptedJob {
    /// UUID do trabalho durável.
    pub job_id: Uuid,
    /// URL relativa de acompanhamento retornada também em Location.
    pub status_url: String,
}

#[utoipa::path(get,path="/api/v1/b3/cotahist/quotes",tag="COTAHIST",summary="Consultar fatos históricos nominais COTAHIST com revisão fixada",description="Consulta registros dos arquivos oficiais COTAHIST anual, mensal e diário da B3.\nPreserva moeda, fator de cotação, distribuição e prazo; não aplica ajustes corporativos.\nFiltros temporais são inclusivos; equivalência não duplica fatos. A revisão fixa resultados entre páginas.\nToda linha contém proveniência até member/registro e estado da identidade. Não dispara ingestão.",params(QuoteFilters),responses((status=200,description="Página na revisão selecionada",body=QuotePage),(status=400,description="Filtros inválidos",body=super::release_error::ErrorResponse),(status=401,description="Bearer ausente/inválido"),(status=403,description="Fonte/finalidade não elegível"),(status=404,description="Revisão inexistente"),(status=409,description="Conflito de seleção"),(status=500,description="Falha de dependência"),(status=503,description="Nenhuma revisão disponível")),security(("bearer_auth"=[])))]
pub async fn quotes(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteFilters>, QueryRejection>,
) -> Result<Json<QuotePage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de consulta inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .quotes(filters, QueryPurpose::Rest, false)
            .await?,
    ))
}

#[utoipa::path(get,path="/api/v1/b3/cotahist/series",tag="COTAHIST",summary="Consultar série COTAHIST inequívoca com preços nominais sem ajuste",description="Seleciona fatos oficiais da família COTAHIST para ticker, ISIN ou instrumento temporal.\nExige desambiguação se mais de um mercado/distribuição/prazo permanecer no mesmo pregão.\nPREULT significa último negócio publicado, sem redefinir fechamento oficial.\nPaginação, precisão, proveniência e revisão seguem a mesma consulta de fatos; não faz forward-fill ou ingestão.",params(QuoteFilters),responses((status=200,description="Página da série selecionada",body=QuotePage),(status=400,description="Filtros inválidos"),(status=401,description="Não autenticado"),(status=403,description="Finalidade não elegível"),(status=404,description="Revisão inexistente"),(status=409,description="Série ambígua"),(status=500,description="Falha interna"),(status=503,description="Revisão indisponível")),security(("bearer_auth"=[])))]
pub async fn series(
    State(state): State<Arc<AppState>>,
    filters: Result<Query<QuoteFilters>, QueryRejection>,
) -> Result<Json<QuotePage>, ApiError> {
    let Query(filters) = filters.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Parâmetros de consulta inválidos.",
        )
    })?;
    Ok(Json(
        QueryService::new(state.pool.clone())
            .quotes(filters, QueryPurpose::Rest, true)
            .await?,
    ))
}

#[utoipa::path(post,path="/api/v1/b3/jobs",tag="Operação",summary="Enfileirar operação administrativa durável sobre fontes e revisões B3",description="Persiste a intenção em PostgreSQL, sem baixar ou interpretar arquivos durante o request.\nComandos abrangem ingestão, reparse, replay, rebuild, publicação e rollback.\nA mesma chave e comando retorna o mesmo trabalho; pedido incompatível retorna 409.\nRequer credencial administrativa. O operador acompanha fase e resultado pela URL Location.",request_body=SubmitJob,responses((status=202,description="Trabalho aceito",body=AcceptedJob),(status=200,description="Consulta do trabalho pela localização indicada"),(status=400,description="Pedido inválido"),(status=401,description="Não autenticado"),(status=403,description="Credencial sem administração"),(status=404,description="Recurso referenciado ausente durante execução"),(status=409,description="Conflito idempotente ou de escopo"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn submit_job(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<SubmitJob>, JsonRejection>,
) -> Result<(StatusCode, HeaderMap, Json<AcceptedJob>), ApiError> {
    let Json(payload) = payload.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Comando administrativo inválido.",
        )
    })?;
    let id = JobService::new(state.pool.clone())
        .submit(
            &payload.idempotency_key,
            payload.command,
            payload.priority,
            payload.max_attempts.unwrap_or(3),
        )
        .await?;
    let status_url = format!("/api/v1/b3/jobs/{id}");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::LOCATION,
        status_url.parse().map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "LOCATION_FAILED",
                "Falha ao construir acompanhamento.",
            )
        })?,
    );
    Ok((
        StatusCode::ACCEPTED,
        headers,
        Json(AcceptedJob {
            job_id: id,
            status_url,
        }),
    ))
}

#[utoipa::path(get,path="/api/v1/b3/jobs/{id}",tag="Operação",summary="Acompanhar trabalho administrativo, tentativas e heartbeat",description="Lê o estado durável PostgreSQL de operação sobre fontes oficiais B3.\nExpõe apenas comando fechado e códigos de erro seguros, sem SQL ou tokens.\nNão interpreta arquivos nem executa retries durante a consulta; leitura não altera o trabalho.\nRequer credencial administrativa.",params(("id"=Uuid,Path,description="UUID do trabalho")),responses((status=200,description="Estado durável",body=JobView),(status=400,description="ID inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Trabalho inexistente"),(status=409,description="Não aplicável à leitura"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn job(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
) -> Result<Json<JobView>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    Ok(Json(QueryService::new(state.pool.clone()).job(id).await?))
}

#[utoipa::path(post,path="/api/v1/b3/jobs/{id}/cancel",tag="Operação",summary="Solicitar cancelamento durável de trabalho administrativo",description="Persiste cancelamento de job enfileirado ou em execução.\nExecutores verificam a solicitação nos limites de download/member/lote/publicação.\nConfirmações anteriores preservam sua evidência; não apaga raw nem auditoria.\nOperação idempotente e exclusiva de administração; não garante interrupção antes de um commit que já venceu a corrida.",params(("id"=Uuid,Path,description="UUID do trabalho")),responses((status=202,description="Solicitação registrada",body=AcceptedJob),(status=200,description="Acompanhamento disponível em Location"),(status=400,description="ID inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Trabalho inexistente"),(status=409,description="Trabalho já terminal"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn cancel_job(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
) -> Result<(StatusCode, Json<AcceptedJob>), ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    QueryService::new(state.pool.clone()).job(id).await?;
    if !JobService::new(state.pool.clone())
        .request_cancel(id)
        .await?
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "JOB_TERMINAL",
            "Trabalho já encerrado.",
        ));
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(AcceptedJob {
            job_id: id,
            status_url: format!("/api/v1/b3/jobs/{id}"),
        }),
    ))
}
