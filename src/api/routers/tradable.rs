use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{InternalMetadata, PaginatedResponse, PaginationParams, Provenance};
use crate::api::AppState;

/// Registro diário de ativo elegível para negociação na B3 (Tradable Security List).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TradableSecurityItem {
    /// Identificador único interno (UUID) do registro.
    #[schema(example = "71a9bc34-9812-4012-bcde-334455667788")]
    pub id: String,

    /// UUID do instrumento pai na tabela `b3_instruments` (quando resolvido com sucesso).
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub instrument_id: Option<String>,

    /// Data de referência da listagem de negociáveis (formato YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub reference_date: NaiveDate,

    /// Security ID associado na B3 (quando aplicável).
    #[schema(example = "PETR4")]
    pub security_id: Option<String>,

    /// Código de negociação do ativo na sessão (ticker).
    #[schema(example = "PETR4")]
    pub ticker_symbol: Option<String>,

    /// Código do mercado de negociação (ex.: `EQ`, `OP`, `FU`).
    #[schema(example = "EQ")]
    pub market_code: Option<String>,

    /// Código do segmento operacional (ex.: `CASH`).
    #[schema(example = "CASH")]
    pub segment_code: Option<String>,

    /// Categoria da classe de segurança (ex.: `SHARES`, `FUNDS`).
    #[schema(example = "SHARES")]
    pub security_category: Option<String>,

    /// Data inicial autorizada para negociação do lote.
    #[schema(example = "2000-01-03")]
    pub trading_start_date: Option<NaiveDate>,

    /// Data limite autorizada para negociação.
    #[schema(example = "9999-12-31")]
    pub trading_end_date: Option<NaiveDate>,

    /// Status da amarração cadastral com a tabela raiz de instrumentos (`resolved`, `unresolved`, `ambiguous`).
    #[schema(example = "resolved")]
    pub resolution_status: String,

    /// Metadados de proveniência do arquivo oficial B3.
    pub provenance: Provenance,

    /// Metadados operacionais internos exibidos quando `include_internal=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

/// Parâmetros de filtro para listagem da Tradable Security List.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TradableFilterParams {
    /// Filtro exato por data de referência da sessão de mercado (YYYY-MM-DD).
    #[param(example = "2026-03-01")]
    pub reference_date: Option<NaiveDate>,

    /// Filtro por símbolo de negociação (ticker, case-insensitive, ex: `PETR4`).
    #[param(example = "PETR4")]
    pub ticker: Option<String>,

    /// Filtro por código do mercado (ex: `EQ`, `OP`).
    #[param(example = "EQ")]
    pub market_code: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/tradable-securities",
    tag = "B3 Tradable Security List",
    summary = "Listagem de Títulos Negociáveis (Tradable Security List)",
    description = "Retorna lista de ativos habilitados para negociação em sessão conforme arquivo diário Tradable Security List da B3. \n\n### Regras de Negócio:\n- Permite filtrar por data de sessão (`reference_date`), ticker e código de mercado.\n- Itens não encontrados imediatamente na base de instrumentos permanecem em staging/quarentena até resolução posterior.\n- Cada item carrega o `resolution_status` e proveniência auditável do arquivo CSV processado.",
    params(PaginationParams, TradableFilterParams),
    responses(
        (status = 200, description = "Lista de títulos negociáveis retornada com sucesso.", body = PaginatedResponse<TradableSecurityItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 500, description = "Erro interno ao consultar o banco de dados.")
    )
)]
pub async fn list_tradable_securities(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<TradableFilterParams>,
) -> Result<Json<PaginatedResponse<TradableSecurityItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let market = filters
        .market_code
        .as_deref()
        .map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_tradable_securities t
        WHERE ($1::date IS NULL OR t.reference_date = $1)
          AND ($2::varchar IS NULL OR t.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR t.market_code = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.reference_date)
        .bind(&ticker)
        .bind(&market)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            t.id, t.instrument_id, t.reference_date, t.security_id, t.ticker_symbol,
            t.market_code, t.segment_code, t.security_category,
            t.trading_start_date, t.trading_end_date, t.resolution_status,
            t.source_file_id, t.source_member_name, t.source_line_number,
            t.data_changed_at, t.field_changed_at, t.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_tradable_securities t
        JOIN b3_source_files f ON f.id = t.source_file_id
        WHERE ($1::date IS NULL OR t.reference_date = $1)
          AND ($2::varchar IS NULL OR t.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR t.market_code = $3)
        ORDER BY t.reference_date DESC, t.ticker_symbol ASC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.reference_date)
        .bind(&ticker)
        .bind(&market)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| {
            let provenance = Provenance {
                source_file_type: row.get("source_file_type"),
                source_file_id: row.get("source_file_id"),
                source_file_name: row.get("original_file_name"),
                source_member_name: row.get("source_member_name"),
                reference_date: Some(row.get("reference_date")),
                sha256: row.get("sha256"),
                ingested_at: row.get("ingested_at"),
            };

            let internal_metadata = if pagination.include_internal() {
                Some(InternalMetadata {
                    data_changed_at: row.get("data_changed_at"),
                    field_changed_at: row.get("field_changed_at"),
                    resolution_status: Some(row.get("resolution_status")),
                    last_seen_at: row.get("last_seen_at"),
                    source_line_number: row.get("source_line_number"),
                })
            } else {
                None
            };

            TradableSecurityItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                reference_date: row.get("reference_date"),
                security_id: row.get("security_id"),
                ticker_symbol: row.get("ticker_symbol"),
                market_code: row.get("market_code"),
                segment_code: row.get("segment_code"),
                security_category: row.get("security_category"),
                trading_start_date: row.get("trading_start_date"),
                trading_end_date: row.get("trading_end_date"),
                resolution_status: row.get("resolution_status"),
                provenance,
                internal_metadata,
            }
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
    path = "/api/v1/b3/tradable-securities/by-date/{reference_date}",
    tag = "B3 Tradable Security List",
    summary = "Listagem de Negociáveis por Data de Referência",
    description = "Consulta o catálogo de ativos autorizados para negociação em uma data de sessão específica da B3.",
    params(
        ("reference_date" = NaiveDate, Path, description = "Data da sessão de negociação (YYYY-MM-DD)", example = "2026-03-01"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Lista de títulos da sessão recuperada com sucesso.", body = PaginatedResponse<TradableSecurityItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_tradable_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TradableSecurityItem>>, (StatusCode, String)> {
    let filters = TradableFilterParams {
        reference_date: Some(date),
        ticker: None,
        market_code: None,
    };
    list_tradable_securities(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}",
    tag = "B3 Tradable Security List",
    summary = "Histórico de Elegibilidade por Ticker",
    description = "Recupera todas as ocorrências de elegibilidade para negociação do ticker ao longo das sessões de mercado registradas.",
    params(
        ("ticker_symbol" = String, Path, description = "Símbolo de negociação da B3 (ex: PETR4)", example = "PETR4"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Histórico de elegibilidade retornado com sucesso.", body = PaginatedResponse<TradableSecurityItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_tradable_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<TradableSecurityItem>>, (StatusCode, String)> {
    let filters = TradableFilterParams {
        reference_date: None,
        ticker: Some(ticker),
        market_code: None,
    };
    list_tradable_securities(State(state), Query(pagination), Query(filters)).await
}
