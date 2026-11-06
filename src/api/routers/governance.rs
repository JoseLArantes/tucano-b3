use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{
    CoverageMetric, PaginatedResponse, PaginationParams, QuarantineReplayResponse,
};
use crate::api::AppState;

/// Registro isolado na quarentena devido a erro de parsing, incompatibilidade de tipo ou anomalia estrutural.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuarantineItem {
    /// Identificador único (UUID) do item na quarentena.
    #[schema(example = "a1b2c3d4-e5f6-7890-abcd-ef0123456789")]
    pub id: String,

    /// UUID da execução de sincronização que identificou a falha.
    #[schema(example = "f54b6c31-9a72-4d56-8e12-32a76fbc9081")]
    pub sync_run_id: Option<String>,

    /// UUID do arquivo fonte de origem.
    #[schema(example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")]
    pub source_file_id: Option<String>,

    /// Nome do membro descompactado dentro do arquivo container ZIP (se aplicável).
    #[schema(example = "COTAHIST_D01032026.TXT")]
    pub source_member_name: Option<String>,

    /// Família oficial da fonte de dados (ex.: `COTAHIST_DAILY`).
    #[schema(example = "COTAHIST_DAILY")]
    pub source_file_type: String,

    /// Data de referência do arquivo de origem (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub reference_date: Option<NaiveDate>,

    /// Número sequencial da linha física com erro no arquivo original.
    #[schema(example = 4521)]
    pub source_line_number: Option<i32>,

    /// Código categórico padronizado da falha (ex.: `INVALID_LINE_WIDTH`, `DECIMAL_PARSE_ERROR`).
    #[schema(example = "INVALID_LINE_WIDTH")]
    pub reason_code: String,

    /// Mensagem técnica detalhada da exceção ou erro de validação.
    #[schema(example = "Expected exactly 245 bytes for COTAHIST record, found 180")]
    pub error_message: String,

    /// Conteúdo ou payload original da linha com falha preservado para depuração e replay.
    pub raw_payload: serde_json::Value,

    /// Status do registro na quarentena (`quarantined`, `reprocessed`, `discarded`).
    #[schema(example = "quarantined")]
    pub resolution_status: String,

    /// Timestamp UTC em que o registro foi isolado na quarentena.
    #[schema(example = "2026-03-01T22:05:00Z")]
    pub created_at: DateTime<Utc>,
}

/// Registro imutável de alteração de campo de negócio para auditoria e linhagem temporal (Field Changes).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldChangeItem {
    /// Identificador único (UUID) do registro de auditoria.
    #[schema(example = "e5f6a7b8-c9d0-1234-5678-9abcdef01234")]
    pub id: String,

    /// UUID da execução de sincronização que detectou a alteração.
    #[schema(example = "f54b6c31-9a72-4d56-8e12-32a76fbc9081")]
    pub sync_run_id: Option<String>,

    /// UUID do arquivo físico que continha a nova versão dos dados.
    #[schema(example = "d87f7b2a-1c34-4b89-a2de-5c94e019310a")]
    pub source_file_id: Option<String>,

    /// Nome da tabela de negócio afetada (ex.: `b3_instruments`, `b3_cotahist_quotes`).
    #[schema(example = "b3_instruments")]
    pub table_name: String,

    /// Chave primária do registro modificado.
    #[schema(example = "PETR4")]
    pub record_pk: String,

    /// Nome exato da coluna/campo que sofreu alteração.
    #[schema(example = "asset_description")]
    pub field_name: String,

    /// Valor anterior em representação textual (ou nulo se for inserção inicial).
    #[schema(example = "PETROLEO BRASILEIRO S.A.")]
    pub old_value_text: Option<String>,

    /// Novo valor atualizado em representação textual.
    #[schema(example = "PETROLEO BRASILEIRO S.A. PETROBRAS")]
    pub new_value_text: Option<String>,

    /// Timestamp UTC exato da modificação.
    #[schema(example = "2026-03-01T22:10:00Z")]
    pub changed_at: DateTime<Utc>,
}

/// Registro de atualização detectada pelo Updates Scanner em canais oficiais da B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingUpdateItem {
    /// Identificador único (UUID) do evento de detecção.
    #[schema(example = "b2c3d4e5-f6a7-8901-2345-6789abcdef01")]
    pub id: String,

    /// Tipo oficial da fonte de dados detectada.
    #[schema(example = "BVBG_028_INSTRUMENTS_FILE")]
    pub source_file_type: String,

    /// Data de referência do arquivo disponível (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub reference_date: NaiveDate,

    /// URL oficial detectada pelo scanner.
    #[schema(example = "https://www.b3.com.br/pesquisapregao/download?file=IN260301.zip")]
    pub source_url: Option<String>,

    /// Cabeçalho ETag retornado pelo servidor remoto da B3.
    #[schema(example = "\"65e2b4f1-a1b2c\"")]
    pub remote_etag: Option<String>,

    /// Status da atualização (`pending`, `ingested`, `ignored`).
    #[schema(example = "pending")]
    pub status: String,

    /// Timestamp UTC de quando o scanner detectou a disponibilidade do arquivo.
    #[schema(example = "2026-03-01T21:45:00Z")]
    pub discovered_at: DateTime<Utc>,
}

/// Parâmetros de filtro para o histórico de alterações de campo (Field Changes).
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct FieldChangeFilterParams {
    /// Filtrar pela tabela afetada (ex.: `b3_instruments`, `b3_cotahist_quotes`).
    #[param(example = "b3_instruments")]
    pub table_name: Option<String>,

    /// Filtrar pela chave primária do registro (ex.: `PETR4`).
    #[param(example = "PETR4")]
    pub record_pk: Option<String>,

    /// Filtrar pelo nome do campo modificado (ex.: `asset_description`).
    #[param(example = "asset_description")]
    pub field_name: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/quarantine",
    tag = "B3 Data Quality & Governance",
    summary = "Listagem de Registros em Quarentena",
    description = "Retorna os registros que foram isolados durante a ingestão por falhas de parsing, linhas truncadas ou incompatibilidade com o esquema. \n\n### Garantia de Resiliência:\n- Linhas com erro são isoladas sem abortar o processamento das linhas válidas do mesmo arquivo.\n- Preserva a linha física exata (`source_line_number`), payload original e o motivo detalhado.",
    params(PaginationParams),
    responses(
        (status = 200, description = "Registros em quarentena retornados com sucesso.", body = PaginatedResponse<QuarantineItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_quarantine_records(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<QuarantineItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_quarantine_records")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT id, sync_run_id, source_file_id, source_member_name, source_file_type, reference_date, source_line_number, reason_code, error_message, raw_payload, resolution_status, created_at
        FROM b3_quarantine_records
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| QuarantineItem {
            id: r.get("id"),
            sync_run_id: r.get("sync_run_id"),
            source_file_id: r.get("source_file_id"),
            source_member_name: r.get("source_member_name"),
            source_file_type: r.get("source_file_type"),
            reference_date: r.get("reference_date"),
            source_line_number: r.get("source_line_number"),
            reason_code: r.get("reason_code"),
            error_message: r.get("error_message"),
            raw_payload: r.get("raw_payload"),
            resolution_status: r.get("resolution_status"),
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
    post,
    path = "/api/v1/b3/quarantine/replay",
    tag = "B3 Data Quality & Governance",
    summary = "Reprocessar Registros da Quarentena (Replay)",
    description = "Marca todos os registros elegíveis da quarentena (`resolution_status = 'quarantined'`) para reprocessamento após correções estruturais ou de regras de negócio.",
    responses(
        (status = 200, description = "Registros da quarentena marcados para reprocessamento com sucesso.", body = QuarantineReplayResponse),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn replay_quarantine(
    State(state): State<Arc<AppState>>,
) -> Result<Json<QuarantineReplayResponse>, (StatusCode, String)> {
    let rows = sqlx::query(
        r#"
        UPDATE b3_quarantine_records
        SET resolution_status = 'reprocessed', reprocessed_at = NOW()
        WHERE resolution_status = 'quarantined'
        RETURNING id
        "#,
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let count = rows.len() as i32;
    Ok(Json(QuarantineReplayResponse {
        reprocessed_count: count,
        message: format!("Replayed {count} quarantine records."),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/field-changes",
    tag = "B3 Data Quality & Governance",
    summary = "Auditoria de Campos Alterados (Field Changes)",
    description = "Retorna o log imutável de alterações granulares de campo em `b3_field_changes`. \n\n### Princípio de Idempotência e Auditoria:\n- Arquivos reprocessados sem alteração de conteúdo produzem zero linhas de auditoria.\n- Apenas os atributos de negócio efetivamente alterados são registrados, com seus valores anterior (`old_value_text`) e novo (`new_value_text`).",
    params(PaginationParams, FieldChangeFilterParams),
    responses(
        (status = 200, description = "Log de alterações retornado com sucesso.", body = PaginatedResponse<FieldChangeItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_field_changes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<FieldChangeFilterParams>,
) -> Result<Json<PaginatedResponse<FieldChangeItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let count_query = r#"
        SELECT count(*)
        FROM b3_field_changes
        WHERE ($1::varchar IS NULL OR table_name = $1)
          AND ($2::varchar IS NULL OR record_pk = $2)
          AND ($3::varchar IS NULL OR field_name = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&filters.table_name)
        .bind(&filters.record_pk)
        .bind(&filters.field_name)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT id, sync_run_id, source_file_id, table_name, record_pk, field_name, old_value_text, new_value_text, changed_at
        FROM b3_field_changes
        WHERE ($1::varchar IS NULL OR table_name = $1)
          AND ($2::varchar IS NULL OR record_pk = $2)
          AND ($3::varchar IS NULL OR field_name = $3)
        ORDER BY changed_at DESC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(&filters.table_name)
        .bind(&filters.record_pk)
        .bind(&filters.field_name)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| FieldChangeItem {
            id: r.get("id"),
            sync_run_id: r.get("sync_run_id"),
            source_file_id: r.get("source_file_id"),
            table_name: r.get("table_name"),
            record_pk: r.get("record_pk"),
            field_name: r.get("field_name"),
            old_value_text: r.get("old_value_text"),
            new_value_text: r.get("new_value_text"),
            changed_at: r.get("changed_at"),
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
    path = "/api/v1/b3/pending-updates",
    tag = "B3 Data Quality & Governance",
    summary = "Listagem de Atualizações Pendentes (Updates Scanner)",
    description = "Retorna arquivos oficiais da B3 cuja disponibilidade foi detectada pelo scanner (via HTTP HEAD / ETag), aguardando ingestão.",
    params(PaginationParams),
    responses(
        (status = 200, description = "Lista de atualizações pendentes retornada com sucesso.", body = PaginatedResponse<PendingUpdateItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_pending_updates(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PendingUpdateItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM b3_pending_updates")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query(
        r#"
        SELECT id, source_file_type, reference_date, source_url, remote_etag, status, discovered_at
        FROM b3_pending_updates
        ORDER BY reference_date DESC, discovered_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| PendingUpdateItem {
            id: r.get("id"),
            source_file_type: r.get("source_file_type"),
            reference_date: r.get("reference_date"),
            source_url: r.get("source_url"),
            remote_etag: r.get("remote_etag"),
            status: r.get("status"),
            discovered_at: r.get("discovered_at"),
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
    path = "/api/v1/b3/diagnostics/coverage",
    tag = "B3 Data Quality & Governance",
    summary = "Diagnóstico de Cobertura e Limites Temporais",
    description = "Retorna métricas consolidadas de cobertura por tabela: data mínima, data máxima, total de registros persistidos e total de arquivos oficiais da B3 processados.",
    responses(
        (status = 200, description = "Métricas de cobertura retornadas com sucesso.", body = Vec<CoverageMetric>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_coverage_diagnostics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CoverageMetric>>, (StatusCode, String)> {
    let queries = vec![
        ("b3_instruments", "SELECT min(created_at::date), max(created_at::date), count(*) FROM b3_instruments"),
        ("b3_tradable_securities", "SELECT min(reference_date), max(reference_date), count(*) FROM b3_tradable_securities"),
        ("b3_price_report_quotes", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_price_report_quotes"),
        ("b3_equities_simplified_price_quotes", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_equities_simplified_price_quotes"),
        ("b3_index_report_values", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_index_report_values"),
        ("b3_index_portfolio_compositions", "SELECT min(composition_date), max(composition_date), count(*) FROM b3_index_portfolio_compositions"),
        ("b3_cotahist_quotes", "SELECT min(trade_date), max(trade_date), count(*) FROM b3_cotahist_quotes"),
    ];

    let mut metrics = Vec::new();

    for (dataset, q) in queries {
        let row = sqlx::query(q)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let min_d: Option<NaiveDate> = row.get(0);
        let max_d: Option<NaiveDate> = row.get(1);
        let total: i64 = row.get(2);

        let files_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM b3_source_files WHERE status = 'ingested'")
                .fetch_one(&state.pool)
                .await
                .unwrap_or(0);

        metrics.push(CoverageMetric {
            dataset: dataset.to_string(),
            min_date: min_d,
            max_date: max_d,
            total_records: total,
            total_files: files_count,
        });
    }

    Ok(Json(metrics))
}
