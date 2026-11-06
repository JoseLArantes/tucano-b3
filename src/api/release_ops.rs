use super::{release_error::ApiError, AppState};
use crate::services::{
    query::*,
    runtime::{discovery::DiscoveryService, RuntimeSettings},
};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

fn pagination(
    filters: Result<Query<ListFilters>, QueryRejection>,
) -> Result<ListFilters, ApiError> {
    filters.map(|Query(p)| p).map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FILTER",
            "Paginação inválida.",
        )
    })
}

macro_rules! collection {
    ($name:ident,$path:literal,$summary:literal,$description:literal,$item:ty,$method:ident) => {
        #[utoipa::path(get,path=$path,tag="Operação",summary=$summary,description=$description,params(ListFilters),responses((status=200,description="Página operacional limitada",body=OperationPage<$item>),(status=400,description="Paginação inválida"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Não aplicável à lista; vazia quando não há itens"),(status=409,description="Não aplicável à inspeção"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
        pub async fn $name(State(state):State<Arc<AppState>>,filters:Result<Query<ListFilters>,QueryRejection>)->Result<Json<OperationPage<$item>>,ApiError>{Ok(Json(QueryService::new(state.pool.clone()).$method(pagination(filters)?).await?))}
    }
}
collection!(source_files,"/api/v1/b3/source-files","Inspecionar artefatos brutos e contadores persistidos por arquivo","Lista arquivos oficiais adquiridos, sua família, canal, SHA-256, parser e contadores.\nA inspeção não implica publicação de todos os registros nem executa reparse.\nURLs tokenizadas e caminhos privados são omitidos.\nOrdenação por criação e ID; limite padrão 100 e máximo 1000.",SourceFileView,source_files);
collection!(pending,"/api/v1/b3/pending-updates","Inspecionar publicações pendentes identificadas pelo scanner","Lista pendências persistidas por descoberta leve de canais oficiais B3.\nUma pendência não comprova alteração de negócio e não dispara aquisição durante consulta.\nAusência de publicação e checagem inconclusiva permanecem distinguíveis no estado operacional.\nOrdenação por detecção e ID, com limite padrão 100/máximo 1000.",PendingView,pending);
collection!(jobs,"/api/v1/b3/jobs","Listar trabalhos administrativos e recursos produzidos","Lista intenções duráveis sobre fontes oficiais e seus estados de execução.\nSucesso de submissão não comprova ingestão ou publicação; outputs identificam recursos persistidos.\nNão executa comandos durante consulta.\nOrdenação por criação e ID, padrão 100/máximo 1000.",JobView,jobs);
collection!(sync_runs,"/api/v1/b3/sync-runs","Inspecionar execuções administrativas do pipeline histórico","Projeção tipada dos mesmos trabalhos duráveis da API e CLI.\nMostra tentativas, heartbeat, códigos seguros e recursos produzidos, sem executar ingestão.\nNão usa runs antigos como evidência de publicação no modelo novo.\nOrdenação por criação e ID, padrão 100/máximo 1000.",JobView,jobs);
collection!(capabilities,"/api/v1/b3/capabilities","Consultar famílias oficiais implementadas e explicitamente adiadas","Catálogo de capacidades de fontes B3 do lançamento 0.0.1.\nHabilitação exige canal configurado e elegibilidade para REST/terceiros; não comprova cobertura ou dados publicados.\n086/186, índices e carteiras permanecem deferred, sem fallback entre famílias.\nA consulta não baixa arquivos nem altera configuração.",Capability,capabilities);

#[utoipa::path(get,path="/api/v1/b3/data-revisions/{id}",tag="Operação",summary="Inspecionar construção, qualidade e publicação de uma revisão",description="Obtém estado de uma revisão COTAHIST e seu período declarado.\nDistingue candidato, revisão pronta, publicada e inválida; mostra rejeições e conflitos sem apagar evidência.\nNão promove resultados nem executa ingestão ou rebuild durante consulta.\nRequer credencial administrativa.",params(("id"=Uuid,Path,description="UUID da revisão")),responses((status=200,description="Estado do candidato/revisão",body=RevisionView),(status=400,description="ID inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Revisão inexistente"),(status=409,description="Não aplicável à inspeção"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn revision(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
) -> Result<Json<RevisionView>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    Ok(Json(
        QueryService::new(state.pool.clone()).revision(id).await?,
    ))
}

macro_rules! revision_collection {
    ($name:ident,$path:literal,$summary:literal,$description:literal,$item:ty,$method:ident) => {
        #[utoipa::path(get,path=$path,tag="Operação",summary=$summary,description=$description,params(("id"=Uuid,Path,description="UUID da revisão"),ListFilters),responses((status=200,description="Página operacional limitada",body=OperationPage<$item>),(status=400,description="ID/filtro inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Revisão inexistente"),(status=409,description="Não aplicável à leitura"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
        pub async fn $name(State(state):State<Arc<AppState>>,id:Result<Path<Uuid>,axum::extract::rejection::PathRejection>,filters:Result<Query<ListFilters>,QueryRejection>)->Result<Json<OperationPage<$item>>,ApiError>{let Path(id)=id.map_err(|_|ApiError::invalid_path())?;Ok(Json(QueryService::new(state.pool.clone()).$method(id,pagination(filters)?).await?))}
    }
}
revision_collection!(conflicts,"/api/v1/b3/data-revisions/{id}/conflicts","Inspecionar versões divergentes sem seleção arbitrária","Lista fatos COTAHIST com versões divergentes sem precedência admissível.\nMantém referências à versão anterior e recebida, member e registro.\nA ordem da lista não define autoridade da fonte; não aplica último download vence.\nA consulta não resolve nem publica conflitos.",ConflictView,conflicts);
revision_collection!(rejections,"/api/v1/b3/data-revisions/{id}/quarantine","Inspecionar rejeições localizadas e recuperáveis da revisão","Lista registros isolados pelo parser com reason code, member, índice físico e offset disponível.\nPayload bruto não é exposto nesta inspeção comum.\nReplay é comando administrativo durável separado e preserva a rejeição original.\nEsta leitura não muda o estado de resolução.",RejectionView,rejections);

#[utoipa::path(get,path="/api/v1/b3/source-files/{id}/members",tag="Operação",summary="Inspecionar inventário determinístico de members e containers aninhados",description="Lista members originais de um artefato oficial B3, incluindo entradas excluídas.\nPreserva índice, pai, nome lógico, SHA-256, tamanho e motivo de exclusão.\nNomes iguais em containers distintos permanecem entradas diferentes.\nConsulta limitada, sem extração, parsing ou exposição de caminhos privados.",params(("id"=String,Path,description="UUID textual do artefato"),ListFilters),responses((status=200,description="Inventário paginado",body=OperationPage<MemberView>),(status=400,description="Filtro inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Arquivo inexistente"),(status=409,description="Não aplicável à leitura"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn members(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    filters: Result<Query<ListFilters>, QueryRejection>,
) -> Result<Json<OperationPage<MemberView>>, ApiError> {
    Ok(Json(
        QueryService::new(state.pool.clone())
            .members(id, pagination(filters)?)
            .await?,
    ))
}

/// Resultado de uma checagem leve manual; não comprova ingestão/publicação.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DiscoveryResult {
    /// Momento de referência da checagem, UTC.
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// Falso para esta checagem manual; não renova saúde agendada.
    pub scheduled: bool,
}
#[utoipa::path(post,path="/api/v1/b3/discovery/scan",tag="Operação",summary="Executar descoberta leve manual sem ingestão ou publicação",description="Checa os canais oficiais habilitados conforme elegibilidade e capacidades de descoberta.\nRegistra resultados conclusivos/inconclusivos e pendências sem baixar o corpo dos datasets.\nNão executa parsing, rebuild ou promoção e não renova o indicador de descoberta agendada.\nSomente administração pode iniciar a checagem.",responses((status=200,description="Checagem leve registrada",body=DiscoveryResult),(status=400,description="Configuração inválida"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Não aplicável"),(status=409,description="Conflito de coordenação"),(status=500,description="Falha ao registrar checagem")),security(("bearer_auth"=[])))]
pub async fn scan(State(state): State<Arc<AppState>>) -> Result<Json<DiscoveryResult>, ApiError> {
    let checked_at = chrono::Utc::now();
    DiscoveryService::new(
        state.pool.clone(),
        RuntimeSettings::from_settings(&state.settings),
    )
    .scan(Uuid::new_v4(), false, checked_at)
    .await?;
    Ok(Json(DiscoveryResult {
        checked_at,
        scheduled: false,
    }))
}

/// Reconhecimento de publicação já preservada sem reingestão fictícia.
#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AcknowledgePending {
    /// Arquivo publicado do mesmo canal que sustenta o reconhecimento.
    pub source_file_id: Uuid,
    /// Motivo explícito da decisão administrativa.
    pub reason: String,
}
/// Resultado do reconhecimento; não afirma alteração de negócio.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PendingAcknowledged {
    /// Pendência tratada, cujo histórico é preservado.
    pub id: Uuid,
    /// Evidência publicada associada à decisão.
    pub source_file_id: Uuid,
}
#[utoipa::path(post,path="/api/v1/b3/pending-updates/{id}/acknowledge",tag="Operação",summary="Reconhecer referência remota já sustentada por arquivo publicado",description="Associa pendência à evidência publicada do mesmo canal e registra motivo.\nPreserva a checagem original e não inventa contadores de linhas inalteradas.\nReconhecimento não dispensa revalidação periódica nem altera preços.\nExige administração; não executa download, parsing ou publicação.",params(("id"=Uuid,Path,description="UUID da pendência")),request_body=AcknowledgePending,responses((status=200,description="Reconhecimento registrado",body=PendingAcknowledged),(status=400,description="Motivo/arquivo inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Pendência/arquivo inexistente"),(status=409,description="Evidência não publicável ou canal incompatível"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn acknowledge_pending(
    State(state): State<Arc<AppState>>,
    id: Result<Path<Uuid>, axum::extract::rejection::PathRejection>,
    payload: Result<Json<AcknowledgePending>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<PendingAcknowledged>, ApiError> {
    let Path(id) = id.map_err(|_| ApiError::invalid_path())?;
    let Json(payload) = payload.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Reconhecimento inválido.",
        )
    })?;
    if payload.reason.trim().is_empty() || payload.reason.len() > 1024 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REASON",
            "Informe motivo limitado a 1024 bytes.",
        ));
    }
    DiscoveryService::new(
        state.pool.clone(),
        RuntimeSettings::from_settings(&state.settings),
    )
    .acknowledge(id, payload.source_file_id, &payload.reason)
    .await?;
    Ok(Json(PendingAcknowledged {
        id,
        source_file_id: payload.source_file_id,
    }))
}
