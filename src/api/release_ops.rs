use super::{release_error::ApiError, AppState};
use crate::services::{
    query::*,
    runtime::{
        discovery::DiscoveryService,
        discovery_locations::{DiscoveryFileType, DiscoveryLocationKind, DiscoveryLocationOutcome},
        local_channels::{self, LocalChannelRegistration},
        RuntimeSettings,
    },
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
collection!(capabilities,"/api/v1/b3/capabilities","Consultar famílias oficiais implementadas e explicitamente adiadas","Catálogo de capacidades de fontes B3 do lançamento 0.0.1.\nSomente as granularidades COTAHIST implementam import com corte de data de negócio por registro. BVBG.028 e Tradable Security List permanecem adiadas até existir filtro confiável sobre referência interna de cada registro; 086/186, índices e carteiras também permanecem deferred, sem fallback entre famílias.\nHabilitação exige canal configurado e elegibilidade para REST/terceiros; não comprova cobertura ou dados publicados.\nA consulta não baixa arquivos nem altera configuração.",Capability,capabilities);

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
    #[schema(example = "2026-11-08T15:00:00Z")]
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// Falso para esta checagem manual; não renova saúde agendada.
    pub scheduled: bool,
    /// Quantidade de locais configurados e checados.
    #[schema(example = 1)]
    pub locations_scanned: usize,
    /// Total de arquivos COTAHIST identificados dentro dos limites de varredura.
    #[schema(example = 1)]
    pub candidate_count: usize,
    /// Indica se a resposta ou a varredura de algum local foi limitada.
    pub candidates_truncated: bool,
    /// Resultado agregado por local, sem revelar diretórios raiz configurados.
    pub locations: Vec<DiscoveryLocationResult>,
    /// Até 1000 candidatos com metadados. O conteúdo dos arquivos nunca é retornado.
    pub candidates: Vec<DiscoveryCandidateResult>,
}

/// Estado de uma fonte configurada para descoberta.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DiscoveryLocationResult {
    /// Tipo configurado: `folder`, `b3_url` ou `object_storage_bucket`.
    #[schema(example = "folder")]
    pub kind: DiscoveryLocationKind,
    /// Rótulo estável da posição na configuração, sem caminho ou segredo.
    #[schema(example = "folder-1")]
    pub label: String,
    /// Resultado de descoberta: `available`, `absent`, `inconclusive` ou `failed`.
    #[schema(example = "available")]
    pub outcome: DiscoveryLocationOutcome,
    /// Quantidade de candidatos observados neste local.
    pub candidate_count: usize,
    /// Indica que o limite de entradas deste local foi atingido.
    pub truncated: bool,
}

/// Metadados públicos de um possível arquivo COTAHIST encontrado.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DiscoveryCandidateResult {
    /// Família COTAHIST inferida pelo padrão oficial do nome do arquivo.
    #[schema(example = "COTAHIST_YEARLY")]
    pub source_file_type: DiscoveryFileType,
    /// Localizador sem credenciais: URL B3, URI S3 ou caminho relativo rotulado.
    #[schema(example = "folder:1/2025/COTAHIST_A2025.ZIP")]
    pub source_locator: String,
    /// Tamanho declarado pelo sistema de arquivos/armazenamento, em bytes.
    #[schema(example = 12345)]
    pub byte_size: Option<i64>,
    /// ETag remoto, quando publicado pelo servidor; não é tratado como SHA-256.
    #[schema(example = "\"opaque-etag\"")]
    pub remote_etag: Option<String>,
    /// Última modificação declarada pela fonte, quando disponível.
    #[schema(example = "2026-11-08T14:45:00Z")]
    pub remote_last_modified: Option<String>,
}

#[utoipa::path(post,path="/api/v1/b3/discovery/scan",tag="Operação",summary="Executar descoberta leve manual sem ingestão ou publicação",description="Checa os canais oficiais habilitados conforme capacidades e elegibilidade e percorre cada entrada de B3_DISCOVERY_LOCATIONS: pastas, URLs HTTPS diretas de arquivos em hosts B3 permitidos e buckets S3 compatíveis com prefixo opcional.\n\nA busca local não segue links simbólicos; listagens locais e S3 têm limite de 10.000 entradas por local e a resposta inclui até 1.000 candidatos. Nomes COTAHIST anual, mensal e diário são classificados pelo nome; o scanner consulta apenas metadados e não lê, baixa, extrai, faz parsing ou ingere corpos. URLs B3 usam HEAD com validação de host, DNS e redirecionamentos. Buckets usam ListObjectsV2 com a cadeia padrão de credenciais AWS e requerem permissão de leitura de listagem; nenhum GetObject é enviado.\n\nAs observações ficam registradas. Um candidato de pasta/URL/bucket não equivale a canal habilitado, evidência de elegibilidade, pendência de canal, aquisição ou publicação; registre e homologue um canal antes de importar. ETag é metadado remoto, não hash de conteúdo.\n\nA configuração desconhecida ou inválida falha explicitamente. Falhas individuais aparecem como outcome=failed e não impedem a checagem dos outros locais. Não renova o indicador de descoberta agendada. Requer TUCANO_B3_ADMIN_TOKEN.",responses((status=200,description="Checagem e candidatos encontrados",body=DiscoveryResult),(status=400,description="Configuração inválida"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Não aplicável"),(status=409,description="Conflito de coordenação"),(status=500,description="Falha ao registrar checagem")),security(("bearer_auth"=[])))]
pub async fn scan(State(state): State<Arc<AppState>>) -> Result<Json<DiscoveryResult>, ApiError> {
    let checked_at = chrono::Utc::now();
    let report = DiscoveryService::new(
        state.pool.clone(),
        RuntimeSettings::from_settings(&state.settings),
    )
    .scan(Uuid::new_v4(), false, checked_at)
    .await?;
    Ok(Json(DiscoveryResult {
        checked_at,
        scheduled: false,
        locations_scanned: report.locations.len(),
        candidate_count: report.candidate_count,
        candidates_truncated: report.candidates_truncated,
        locations: report
            .locations
            .into_iter()
            .map(|location| DiscoveryLocationResult {
                kind: location.location_kind,
                label: location.location_label,
                outcome: location.outcome,
                candidate_count: location.candidate_count,
                truncated: location.truncated,
            })
            .collect(),
        candidates: report
            .candidates
            .into_iter()
            .map(|candidate| DiscoveryCandidateResult {
                source_file_type: candidate.source_file_type,
                source_locator: candidate.source_locator,
                byte_size: candidate.byte_size,
                remote_etag: candidate.remote_etag,
                remote_last_modified: candidate.remote_last_modified,
            })
            .collect(),
    }))
}

/// Attestation to use one discovered local corpus for internal development only.
#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RegisterLocalCandidate {
    /// Relative locator returned by the most recent manual folder discovery scan.
    #[schema(example = "folder:1/COTAHIST_A2026.ZIP")]
    pub source_locator: String,
    /// Operator attestation limited to manual acquisition, raw retention, and processing.
    /// REST, MCP, and third-party delivery remain ineligible.
    #[schema(example = true)]
    pub authorize_internal_development: bool,
}

/// Registered channel identity and inclusive business-date window inferred from its filename.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct RegisteredLocalCandidate {
    /// Stable channel ID to pass with the ingest job command.
    #[schema(example = "local-89abc0123456789abcdef012")]
    pub channel_id: String,
    /// COTAHIST family inferred from the candidate name.
    #[schema(example = "COTAHIST_YEARLY")]
    pub source_file_type: String,
    /// Sanitized local locator; no absolute filesystem path.
    #[schema(example = "folder:1/COTAHIST_A2026.ZIP")]
    pub source_locator: String,
    /// First date covered by the named publication.
    #[schema(example = "2026-01-01")]
    pub period_start: chrono::NaiveDate,
    /// Last date covered by the named publication.
    #[schema(example = "2026-12-31")]
    pub period_end: chrono::NaiveDate,
    /// True when this channel was already present before this request.
    pub already_registered: bool,
    /// This channel is enabled for internal development only.
    pub internal_development_only: bool,
}

impl From<LocalChannelRegistration> for RegisteredLocalCandidate {
    fn from(value: LocalChannelRegistration) -> Self {
        Self {
            channel_id: value.channel_id,
            source_file_type: value.source_file_type,
            source_locator: value.source_locator,
            period_start: value.period_start,
            period_end: value.period_end,
            already_registered: value.already_registered,
            internal_development_only: true,
        }
    }
}

#[utoipa::path(post,path="/api/v1/b3/discovery/candidates/register-local",tag="Operação",summary="Habilitar candidato local para importação interna autorizada",description="Registra explicitamente um COTAHIST encontrado na pasta local configurada como canal de aquisição manual.\nExige attestation do operador mantenedor para aquisição, retenção bruta e processamento exclusivamente internos; REST, MCP e entrega a terceiros continuam sem elegibilidade.\nAceita somente candidato da última descoberta manual, com até 24 horas, localizado em B3_RAW_DATA_DIR/imports e sem links simbólicos.\nA família e o período vêm do nome COTAHIST; tamanho atual precisa corresponder à observação. A operação é idempotente por família e localizador.\nRequer credencial administrativa.",request_body=RegisterLocalCandidate,responses((status=200,description="Canal local preparado para importação interna",body=RegisteredLocalCandidate),(status=400,description="Attestation, localizador, caminho ou período inválido"),(status=401,description="Não autenticado"),(status=403,description="Administração exigida"),(status=404,description="Candidato ausente na última descoberta manual"),(status=409,description="Arquivo alterado desde a descoberta ou canal incompatível"),(status=500,description="Falha de persistência")),security(("bearer_auth"=[])))]
pub async fn register_local_candidate(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<RegisterLocalCandidate>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<RegisteredLocalCandidate>, ApiError> {
    let Json(payload) = payload.map_err(|_| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Pedido de registro local inválido.",
        )
    })?;
    if !payload.authorize_internal_development {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "ATTESTATION_REQUIRED",
            "Confirme a autorização limitada ao desenvolvimento interno.",
        ));
    }
    let result = local_channels::register_candidate(
        &state.pool,
        &RuntimeSettings::from_settings(&state.settings),
        &payload.source_locator,
    )
    .await;
    let registered = result.map_err(|error| match error {
        crate::error::AppError::Validation(message) => {
            ApiError::new(StatusCode::BAD_REQUEST, "LOCAL_CHANNEL_INVALID", &message)
        }
        crate::error::AppError::NotFound(message) => ApiError::new(
            StatusCode::NOT_FOUND,
            "DISCOVERED_CANDIDATE_NOT_FOUND",
            &message,
        ),
        crate::error::AppError::LockConflict(message) => ApiError::new(
            StatusCode::CONFLICT,
            "DISCOVERED_CANDIDATE_CHANGED",
            &message,
        ),
        other => ApiError::from(other),
    })?;
    Ok(Json(registered.into()))
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
