use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{InternalMetadata, PaginatedResponse, PaginationParams, Provenance};
use crate::api::AppState;

/// Componente de carteira teórica de índice da B3 (quantidade teórica e peso percentual).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexCompositionItem {
    /// Identificador único interno (UUID) do registro.
    #[schema(example = "55a1b2c3-d4e5-6789-a0b1-c2d3e4f5a6b7")]
    pub id: String,

    /// UUID do índice pai na tabela `b3_indices`.
    #[schema(example = "33a4b5c6-d7e8-9012-f3a4-b5c6d7e8f901")]
    pub index_id: Option<String>,

    /// UUID do instrumento associado em `b3_instruments` (quando resolvido com sucesso).
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub instrument_id: Option<String>,

    /// Data de apuração e vigência da carteira teórica (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub composition_date: NaiveDate,

    /// Código do índice da B3 (ex: IBOV, IBrX-100, SMLL).
    #[schema(example = "IBOV")]
    pub index_code: String,

    /// Símbolo de negociação do ativo na carteira (ticker).
    #[schema(example = "PETR4")]
    pub ticker_symbol: Option<String>,

    /// Código ISIN internacional do ativo componente.
    #[schema(example = "BRPETRACNPR6")]
    pub isin: Option<String>,

    /// Nome descritivo da ação ou emissor.
    #[schema(example = "PETROBRAS PN")]
    pub asset_name: Option<String>,

    /// Quantidade teórica de ações atribuída ao ativo na carteira do índice.
    #[schema(example = "4567890123")]
    pub theoretical_quantity: Option<Decimal>,

    /// Peso ponderado do ativo na carteira teórica.
    #[schema(example = "0.0785")]
    pub portfolio_weight: Option<Decimal>,

    /// Participação percentual exata do ativo na composição do índice (ex: 7.85%).
    #[schema(example = "7.850")]
    pub participation_percentage: Option<Decimal>,

    /// Status da amarração cadastral com o instrumento (`resolved`, `unresolved`, `ambiguous`).
    #[schema(example = "resolved")]
    pub resolution_status: String,

    /// Metadados de proveniência do arquivo oficial B3.
    pub provenance: Provenance,

    /// Metadados operacionais internos exibidos quando `include_internal=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

/// Parâmetros de filtro para composição teórica de índices.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IndexCompositionFilterParams {
    /// Data da composição da carteira teórica (YYYY-MM-DD).
    #[param(example = "2026-03-01")]
    pub composition_date: Option<NaiveDate>,

    /// Código do índice (case-insensitive, ex: `IBOV`).
    #[param(example = "IBOV")]
    pub index_code: Option<String>,

    /// Símbolo de negociação da ação componente (ex: `PETR4`).
    #[param(example = "PETR4")]
    pub ticker: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/index-composition/portfolios",
    tag = "B3 Index Portfolio Composition",
    summary = "Listagem de Carteiras Teóricas de Índices",
    description = "Retorna os componentes individuais (ativos, quantidades teóricas e pesos percentuais de participação) das carteiras teóricas oficiais publicadas pela B3. \n\n### Regras:\n- Permite filtrar por data de vigência (`composition_date`), código do índice (`index_code`) e ticker.\n- Decimais são representados com precisão exata sem conversão por floats.",
    params(PaginationParams, IndexCompositionFilterParams),
    responses(
        (status = 200, description = "Composição de carteiras retornada com sucesso.", body = PaginatedResponse<IndexCompositionItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_index_compositions(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<IndexCompositionFilterParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let idx_code = filters
        .index_code
        .as_deref()
        .map(|s| s.trim().to_uppercase());
    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_index_portfolio_compositions c
        WHERE ($1::date IS NULL OR c.composition_date = $1)
          AND ($2::varchar IS NULL OR c.index_code = $2)
          AND ($3::varchar IS NULL OR c.ticker_symbol = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.composition_date)
        .bind(&idx_code)
        .bind(&ticker)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            c.id, c.index_id, c.instrument_id, c.composition_date, c.index_code,
            c.ticker_symbol, c.isin, c.asset_name, c.theoretical_quantity,
            c.portfolio_weight, c.participation_percentage, c.resolution_status,
            c.source_file_id, c.source_member_name, c.source_line_number,
            c.data_changed_at, c.field_changed_at, c.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_index_portfolio_compositions c
        JOIN b3_source_files f ON f.id = c.source_file_id
        WHERE ($1::date IS NULL OR c.composition_date = $1)
          AND ($2::varchar IS NULL OR c.index_code = $2)
          AND ($3::varchar IS NULL OR c.ticker_symbol = $3)
        ORDER BY c.composition_date DESC, c.index_code ASC, c.participation_percentage DESC NULLS LAST
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.composition_date)
        .bind(&idx_code)
        .bind(&ticker)
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
                reference_date: Some(row.get("composition_date")),
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

            IndexCompositionItem {
                id: row.get("id"),
                index_id: row.get("index_id"),
                instrument_id: row.get("instrument_id"),
                composition_date: row.get("composition_date"),
                index_code: row.get("index_code"),
                ticker_symbol: row.get("ticker_symbol"),
                isin: row.get("isin"),
                asset_name: row.get("asset_name"),
                theoretical_quantity: row.get("theoretical_quantity"),
                portfolio_weight: row.get("portfolio_weight"),
                participation_percentage: row.get("participation_percentage"),
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
    path = "/api/v1/b3/index-composition/portfolios/{index_code}",
    tag = "B3 Index Portfolio Composition",
    summary = "Composição de Carteira por Código do Índice",
    description = "Retorna os componentes e pesos da carteira teórica mais recente para o índice informado (ex.: `IBOV`).",
    params(
        ("index_code" = String, Path, description = "Código identificador do índice na B3 (ex: IBOV)", example = "IBOV"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Componentes do índice retornados com sucesso.", body = PaginatedResponse<IndexCompositionItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_index_composition_by_index(
    State(state): State<Arc<AppState>>,
    Path(index_code): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let filters = IndexCompositionFilterParams {
        composition_date: None,
        index_code: Some(index_code),
        ticker: None,
    };
    list_index_compositions(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/index-composition/portfolios/{index_code}/by-date/{composition_date}",
    tag = "B3 Index Portfolio Composition",
    summary = "Composição de Carteira por Índice e Data",
    description = "Retorna a composição da carteira teórica do índice em uma data específica de apuração.",
    params(
        ("index_code" = String, Path, description = "Código identificador do índice na B3 (ex: IBOV)", example = "IBOV"),
        ("composition_date" = NaiveDate, Path, description = "Data da composição da carteira (YYYY-MM-DD)", example = "2026-03-01"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Composição na data retornada com sucesso.", body = PaginatedResponse<IndexCompositionItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_index_composition_by_index_and_date(
    State(state): State<Arc<AppState>>,
    Path((index_code, date)): Path<(String, NaiveDate)>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let filters = IndexCompositionFilterParams {
        composition_date: Some(date),
        index_code: Some(index_code),
        ticker: None,
    };
    list_index_compositions(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/index-composition/components/by-ticker/{ticker_symbol}",
    tag = "B3 Index Portfolio Composition",
    summary = "Participação de um Ticker em Índices B3",
    description = "Localiza todos os índices da B3 nos quais uma ação específica (ticker) participa como componente, com seus respectivos percentuais e quantidades teóricas.",
    params(
        ("ticker_symbol" = String, Path, description = "Símbolo de negociação do ativo (ex: PETR4)", example = "PETR4"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Participações do ticker retornadas com sucesso.", body = PaginatedResponse<IndexCompositionItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_index_composition_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<IndexCompositionItem>>, (StatusCode, String)> {
    let filters = IndexCompositionFilterParams {
        composition_date: None,
        index_code: None,
        ticker: Some(ticker),
    };
    list_index_compositions(State(state), Query(pagination), Query(filters)).await
}
