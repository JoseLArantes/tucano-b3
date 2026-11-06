use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::str::FromStr;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{
    ManualSyncRequest, ManualSyncResponse, PaginatedResponse, PaginationParams,
    RebuildIdentityResponse,
};
use crate::api::AppState;
use crate::db::enums::SourceFileType;
use crate::db::resolver::RelationshipResolver;

/// Registro de auditoria e proveniência de arquivo físico oficial da B3 importado.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceFileItem {
    /// Identificador único (UUID) do arquivo fonte em `b3_source_files`.
    #[schema(example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")]
    pub id: String,

    /// Tipo de arquivo oficial da B3 (ex: `BVBG_028_INSTRUMENTS_FILE`).
    #[schema(example = "BVBG_028_INSTRUMENTS_FILE")]
    pub source_file_type: String,

    /// Nome canônico da família do arquivo.
    #[schema(example = "InstrumentsConsolidated")]
    pub source_name: Option<String>,

    /// Data de referência de mercado do arquivo (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub reference_date: Option<NaiveDate>,

    /// URL externa oficial a partir da qual o download foi executado.
    #[schema(example = "https://www.b3.com.br/pesquisapregao/download?file=IN260301.zip")]
    pub source_url: Option<String>,

    /// Nome original do arquivo físico ou container ZIP baixado.
    #[schema(example = "IN260301.zip")]
    pub original_file_name: Option<String>,

    /// Checksum SHA-256 do arquivo baixado para garantia de integridade.
    #[schema(example = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")]
    pub sha256: Option<String>,

    /// Tamanho exato em bytes do arquivo baixado.
    #[schema(example = 14502394)]
    pub byte_size: Option<i64>,

    /// Timestamp UTC do término do download físico.
    #[schema(example = "2026-03-01T22:00:15Z")]
    pub downloaded_at: Option<DateTime<Utc>>,

    /// Timestamp UTC do término do parsing estrutural.
    #[schema(example = "2026-03-01T22:01:30Z")]
    pub parsed_at: Option<DateTime<Utc>>,

    /// Timestamp UTC de conclusão da ingestão e persistência relacional.
    #[schema(example = "2026-03-01T22:02:10Z")]
    pub ingested_at: Option<DateTime<Utc>>,

    /// Versão do parser streaming que processou o arquivo (ex: `v1`).
    #[schema(example = "v1")]
    pub parser_version: Option<String>,

    /// Status do processamento (`downloaded`, `parsed`, `ingested`, `skipped_duplicate`, `failed`).
    #[schema(example = "ingested")]
    pub status: String,

    /// Quantidade de arquivos membros internos identificados no arquivo (para ZIPs).
    #[schema(example = 1)]
    pub member_count: Option<i32>,

    /// Quantidade de linhas ou nós brutos lidos do arquivo.
    #[schema(example = 120562)]
    pub row_count_raw: Option<i32>,

    /// Quantidade de registros novos inseridos no banco.
    #[schema(example = 120562)]
    pub row_count_inserted: Option<i32>,

    /// Quantidade de registros existentes que sofreram modificação de campos de negócio.
    #[schema(example = 0)]
    pub row_count_changed: Option<i32>,

    /// Quantidade de registros existentes que eram canonicamente idênticos.
    #[schema(example = 0)]
    pub row_count_unchanged: Option<i32>,

    /// Quantidade de registros isolados na quarentena por anomalias ou erros de validação.
    #[schema(example = 0)]
    pub row_count_quarantined: Option<i32>,

    /// Mensagem descritiva de erro quando o status for `failed`.
    #[schema(nullable)]
    pub error_message: Option<String>,

    /// Timestamp UTC de criação do registro de rastreabilidade.
    #[schema(example = "2026-03-01T22:00:00Z")]
    pub created_at: DateTime<Utc>,
}

/// Membro interno descompactado de um container de arquivo ZIP da B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SourceFileMemberItem {
    /// Identificador único (UUID) do membro.
    #[schema(example = "77b8c9d0-e1f2-3456-b7c8-d9e0f1a2b3c4")]
    pub id: String,

    /// UUID do arquivo ZIP pai em `b3_source_files`.
    #[schema(example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")]
    pub source_file_id: String,

    /// Nome interno do membro descompactado (ex: `IN260301.xml`).
    #[schema(example = "IN260301.xml")]
    pub member_name: String,

    /// Índice sequencial do membro dentro do arquivo ZIP.
    #[schema(example = 0)]
    pub member_index: i32,

    /// Checksum SHA-256 específico do conteúdo descompactado deste membro.
    #[schema(example = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")]
    pub sha256: String,

    /// Tamanho descompactado em bytes do membro.
    #[schema(example = 45120300)]
    pub byte_size: Option<i64>,

    /// Status do processamento do membro (`pending`, `processed`, `failed`).
    #[schema(example = "processed")]
    pub status: String,

    /// Quantidade de linhas inseridas por este membro.
    #[schema(example = 120562)]
    pub row_count_inserted: Option<i32>,

    /// Quantidade de linhas modificadas por este membro.
    #[schema(example = 0)]
    pub row_count_changed: Option<i32>,

    /// Quantidade de linhas idênticas sem alteração por este membro.
    #[schema(example = 0)]
    pub row_count_unchanged: Option<i32>,

    /// Timestamp UTC de quando o processamento deste membro foi concluído.
    #[schema(example = "2026-03-01T22:02:00Z")]
    pub processed_at: DateTime<Utc>,
}

/// Registro de execução de job de sincronização (agendado ou manual).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncRunItem {
    /// Identificador único (UUID) da execução em `b3_sync_runs`.
    #[schema(example = "f54b6c31-9a72-4d56-8e12-32a76fbc9081")]
    pub id: String,

    /// Tipo de trabalho executado (ex: `scheduled_daily`, `manual_sync`, `cotahist_backfill`, `rebuild_identity`).
    #[schema(example = "manual_sync")]
    pub job_type: String,

    /// Identificação do solicitante (ex: `worker_daemon`, `api_user`).
    #[schema(example = "api_user")]
    pub requested_by: Option<String>,

    /// Timestamp UTC de início da execução.
    #[schema(example = "2026-03-01T22:00:00Z")]
    pub started_at: Option<DateTime<Utc>>,

    /// Timestamp UTC de término da execução.
    #[schema(example = "2026-03-01T22:02:15Z")]
    pub finished_at: Option<DateTime<Utc>>,

    /// Status final do job (`running`, `completed`, `failed`).
    #[schema(example = "completed")]
    pub status: String,

    /// Família oficial da fonte de dados processada.
    #[schema(example = "BVBG_028_INSTRUMENTS_FILE")]
    pub source_file_type: Option<String>,

    /// Data inicial do intervalo de referência.
    #[schema(example = "2026-03-01")]
    pub reference_date_from: Option<NaiveDate>,

    /// Data final do intervalo de referência.
    #[schema(example = "2026-03-01")]
    pub reference_date_to: Option<NaiveDate>,

    /// Estatísticas detalhadas consolidadas da execução em formato JSON.
    pub stats_json: serde_json::Value,

    /// Timestamp UTC de criação do registro de execução.
    #[schema(example = "2026-03-01T22:00:00Z")]
    pub created_at: DateTime<Utc>,
}

/// Filtros para listagem de arquivos fonte cadastrados.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SourceFileFilterParams {
    /// Filtro por família de arquivo (ex: `BVBG_028_INSTRUMENTS_FILE`).
    #[param(example = "BVBG_028_INSTRUMENTS_FILE")]
    pub source_file_type: Option<String>,

    /// Filtro por data de referência de mercado (YYYY-MM-DD).
    #[param(example = "2026-03-01")]
    pub reference_date: Option<NaiveDate>,

    /// Filtro por status de ingestão (`ingested`, `skipped_duplicate`, `failed`).
    #[param(example = "ingested")]
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/source-files",
    tag = "B3 Source Files",
    summary = "Listagem de Arquivos Fonte Cadastrados",
    description = "Retorna histórico de arquivos oficiais da B3 baixados e processados pelo sistema, incluindo checksums SHA-256, tamanhos em bytes e contadores de linhas inseridas/modificadas.",
    params(PaginationParams, SourceFileFilterParams),
    responses(
        (status = 200, description = "Lista de arquivos fonte retornada com sucesso.", body = PaginatedResponse<SourceFileItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_source_files(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<SourceFileFilterParams>,
) -> Result<Json<PaginatedResponse<SourceFileItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let count_query = r#"
        SELECT count(*)
        FROM b3_source_files
        WHERE ($1::varchar IS NULL OR source_file_type = $1)
          AND ($2::date IS NULL OR reference_date = $2)
          AND ($3::varchar IS NULL OR status = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&filters.source_file_type)
        .bind(filters.reference_date)
        .bind(&filters.status)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            id, source_file_type, source_name, reference_date, source_url,
            original_file_name, sha256, byte_size, downloaded_at, parsed_at,
            ingested_at, parser_version, status, member_count, row_count_raw,
            row_count_inserted, row_count_changed, row_count_unchanged,
            row_count_quarantined, error_message, created_at
        FROM b3_source_files
        WHERE ($1::varchar IS NULL OR source_file_type = $1)
          AND ($2::date IS NULL OR reference_date = $2)
          AND ($3::varchar IS NULL OR status = $3)
        ORDER BY reference_date DESC NULLS LAST, created_at DESC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(&filters.source_file_type)
        .bind(filters.reference_date)
        .bind(&filters.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| SourceFileItem {
            id: r.get("id"),
            source_file_type: r.get("source_file_type"),
            source_name: r.get("source_name"),
            reference_date: r.get("reference_date"),
            source_url: r.get("source_url"),
            original_file_name: r.get("original_file_name"),
            sha256: r.get("sha256"),
            byte_size: r.get("byte_size"),
            downloaded_at: r.get("downloaded_at"),
            parsed_at: r.get("parsed_at"),
            ingested_at: r.get("ingested_at"),
            parser_version: r.get("parser_version"),
            status: r.get("status"),
            member_count: r.get("member_count"),
            row_count_raw: r.get("row_count_raw"),
            row_count_inserted: r.get("row_count_inserted"),
            row_count_changed: r.get("row_count_changed"),
            row_count_unchanged: r.get("row_count_unchanged"),
            row_count_quarantined: r.get("row_count_quarantined"),
            error_message: r.get("error_message"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
        disclaimer: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/source-files/{id}",
    tag = "B3 Source Files",
    summary = "Obter Arquivo Fonte por UUID",
    description = "Recupera metadados detalhados de integridade, contadores de ingestão e proveniência de um arquivo B3 específico.",
    params(
        ("id" = String, Path, description = "UUID do arquivo fonte em `b3_source_files`", example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")
    ),
    responses(
        (status = 200, description = "Arquivo fonte localizado com sucesso.", body = SourceFileItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Arquivo fonte não encontrado.")
    )
)]
pub async fn get_source_file_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SourceFileItem>, (StatusCode, String)> {
    let row = sqlx::query(
        r#"
        SELECT 
            id, source_file_type, source_name, reference_date, source_url,
            original_file_name, sha256, byte_size, downloaded_at, parsed_at,
            ingested_at, parser_version, status, member_count, row_count_raw,
            row_count_inserted, row_count_changed, row_count_unchanged,
            row_count_quarantined, error_message, created_at
        FROM b3_source_files
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Source file not found".to_string()))?;

    Ok(Json(SourceFileItem {
        id: row.get("id"),
        source_file_type: row.get("source_file_type"),
        source_name: row.get("source_name"),
        reference_date: row.get("reference_date"),
        source_url: row.get("source_url"),
        original_file_name: row.get("original_file_name"),
        sha256: row.get("sha256"),
        byte_size: row.get("byte_size"),
        downloaded_at: row.get("downloaded_at"),
        parsed_at: row.get("parsed_at"),
        ingested_at: row.get("ingested_at"),
        parser_version: row.get("parser_version"),
        status: row.get("status"),
        member_count: row.get("member_count"),
        row_count_raw: row.get("row_count_raw"),
        row_count_inserted: row.get("row_count_inserted"),
        row_count_changed: row.get("row_count_changed"),
        row_count_unchanged: row.get("row_count_unchanged"),
        row_count_quarantined: row.get("row_count_quarantined"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/source-files/{id}/members",
    tag = "B3 Source Files",
    summary = "Listar Membros de um Arquivo Container",
    description = "Retorna todos os arquivos membros internos (arquivos XML, CSV ou TXT descompactados de um arquivo ZIP container oficial da B3).",
    params(
        ("id" = String, Path, description = "UUID do arquivo container em `b3_source_files`", example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")
    ),
    responses(
        (status = 200, description = "Membros do arquivo retornados com sucesso.", body = Vec<SourceFileMemberItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_source_file_members(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SourceFileMemberItem>>, (StatusCode, String)> {
    let rows = sqlx::query(
        r#"
        SELECT id, source_file_id, member_name, member_index, sha256, byte_size, status, row_count_inserted, row_count_changed, row_count_unchanged, processed_at
        FROM b3_source_file_members
        WHERE source_file_id = $1
        ORDER BY member_index ASC
        "#,
    )
    .bind(&id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| SourceFileMemberItem {
            id: r.get("id"),
            source_file_id: r.get("source_file_id"),
            member_name: r.get("member_name"),
            member_index: r.get("member_index"),
            sha256: r.get("sha256"),
            byte_size: r.get("byte_size"),
            status: r.get("status"),
            row_count_inserted: r.get("row_count_inserted"),
            row_count_changed: r.get("row_count_changed"),
            row_count_unchanged: r.get("row_count_unchanged"),
            processed_at: r.get("processed_at"),
        })
        .collect();

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/sync-runs",
    tag = "B3 Sync Jobs",
    summary = "Listagem de Execuções de Sincronização",
    description = "Retorna histórico paginado de execuções de sincronização (jobs diários automáticos do worker, acionamentos manuais e backfills).",
    params(PaginationParams),
    responses(
        (status = 200, description = "Lista de execuções retornada com sucesso.", body = PaginatedResponse<SyncRunItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_sync_runs(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<SyncRunItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let count_query = "SELECT count(*) FROM b3_sync_runs";
    let total: i64 = sqlx::query_scalar(count_query)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT id, job_type, requested_by, started_at, finished_at, status, source_file_type, reference_date_from, reference_date_to, stats_json, created_at
        FROM b3_sync_runs
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
    "#;

    let rows = sqlx::query(select_query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| SyncRunItem {
            id: r.get("id"),
            job_type: r.get("job_type"),
            requested_by: r.get("requested_by"),
            started_at: r.get("started_at"),
            finished_at: r.get("finished_at"),
            status: r.get("status"),
            source_file_type: r.get("source_file_type"),
            reference_date_from: r.get("reference_date_from"),
            reference_date_to: r.get("reference_date_to"),
            stats_json: r.get("stats_json"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total,
        limit,
        offset,
        disclaimer: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/sync-runs/{id}",
    tag = "B3 Sync Jobs",
    summary = "Obter Execução de Sincronização por UUID",
    description = "Recupera os detalhes completos, timestamps e estatísticas consolidadas de uma execução de job específica.",
    params(
        ("id" = String, Path, description = "UUID da execução em `b3_sync_runs`", example = "f54b6c31-9a72-4d56-8e12-32a76fbc9081")
    ),
    responses(
        (status = 200, description = "Execução localizada com sucesso.", body = SyncRunItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Execução não encontrada.")
    )
)]
pub async fn get_sync_run_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SyncRunItem>, (StatusCode, String)> {
    let row = sqlx::query(
        r#"
        SELECT id, job_type, requested_by, started_at, finished_at, status, source_file_type, reference_date_from, reference_date_to, stats_json, created_at
        FROM b3_sync_runs
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Sync run not found".to_string()))?;

    Ok(Json(SyncRunItem {
        id: row.get("id"),
        job_type: row.get("job_type"),
        requested_by: row.get("requested_by"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        status: row.get("status"),
        source_file_type: row.get("source_file_type"),
        reference_date_from: row.get("reference_date_from"),
        reference_date_to: row.get("reference_date_to"),
        stats_json: row.get("stats_json"),
        created_at: row.get("created_at"),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/b3/sync-runs",
    tag = "B3 Sync Jobs",
    summary = "Disparar Sincronização Manual de Arquivos B3",
    description = "Dispara a execução sob demanda da ingestão de uma família oficial da B3 para um intervalo de datas. \n\n### Coordenação por Travas Consultivas (Zero Redis):\n- O sistema adquire uma trava consultiva PostgreSQL a nível de sessão (`pg_try_advisory_lock`).\n- Tentativas simultâneas para o mesmo arquivo/data retornam imediatamente HTTP `409 Conflict`.\n- O parâmetro `force_reparse=true` descarta dados anteriores da data de forma transacional e re-executa a ingestão.",
    request_body = ManualSyncRequest,
    responses(
        (status = 200, description = "Sincronização concluída com sucesso.", body = ManualSyncResponse),
        (status = 400, description = "Requisição inválida (tipo de arquivo desconhecido ou parâmetros inconsistentes)."),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 409, description = "Conflito: Processo de sincronização já em execução para este arquivo/data por outro worker ou usuário."),
        (status = 500, description = "Erro interno durante o processamento do pipeline.")
    )
)]
pub async fn trigger_sync_run(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ManualSyncRequest>,
) -> Result<Json<ManualSyncResponse>, (StatusCode, String)> {
    let sft = SourceFileType::from_str(&payload.source_file_type).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid source_file_type: {e}"),
        )
    })?;

    let force_dl = payload.force_download.unwrap_or(false);
    let force_rp = payload.force_reparse.unwrap_or(false);

    let outcome = state
        .sync_service
        .run_manual_sync(
            sft,
            payload.reference_date_from,
            payload.reference_date_to,
            payload.source_url,
            None,
            force_dl,
            force_rp,
            Some("api_user".to_string()),
        )
        .await
        .map_err(|err| match err {
            crate::error::AppError::LockConflict(msg) => (StatusCode::CONFLICT, msg),
            crate::error::AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            other => (StatusCode::INTERNAL_SERVER_ERROR, other.to_string()),
        })?;

    Ok(Json(ManualSyncResponse {
        sync_run_id: outcome.sync_run_id,
        status: outcome.status.to_string(),
        inserted: outcome.inserted,
        changed: outcome.changed,
        unchanged: outcome.unchanged,
        unresolved: outcome.unresolved,
        warnings: outcome.warnings,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/b3/sync-runs/rebuild-identity",
    tag = "B3 Sync Jobs",
    summary = "Reconstrução Retroativa de Identidades (Rebuild Identity)",
    description = "Reavalia todos os registros com status `unresolved` em todas as tabelas dependentes (COTAHIST, PriceReport, Composição de Índices) contra o catálogo atual de instrumentos. \n\n### Propósito:\n- Resolve registros que foram ingeridos antes da definição cadastral correspondente ser conhecida.\n- Promove registros elegíveis sem alterar preços históricos ou proveniência original.",
    responses(
        (status = 200, description = "Reconstrução concluída com sucesso.", body = RebuildIdentityResponse),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 500, description = "Erro interno ao reconstruir relacionamentos.")
    )
)]
pub async fn rebuild_identity(
    State(state): State<Arc<AppState>>,
) -> Result<Json<RebuildIdentityResponse>, (StatusCode, String)> {
    let resolver = RelationshipResolver::new(&state.pool);
    let resolved = resolver
        .rebuild_unresolved_identities()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RebuildIdentityResponse {
        resolved_count: resolved as i32,
        message: format!(
            "Rebuilt identity relationships; resolved {resolved} previously pending items."
        ),
    }))
}
