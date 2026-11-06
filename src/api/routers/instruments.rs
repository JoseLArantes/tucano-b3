use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;
use utoipa::{IntoParams, ToSchema};

use crate::api::schemas::{InternalMetadata, PaginatedResponse, PaginationParams, Provenance};
use crate::api::AppState;

/// Representação cadastral canônica de um instrumento financeiro registrado na B3.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstrumentItem {
    /// Identificador único interno (UUID) do instrumento.
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,

    /// Identificador oficial do instrumento na estrutura de negociação da B3 (ex: PETR4, VALE3, DI1F27).
    #[schema(example = "PETR4")]
    pub security_id: String,

    /// Fonte da definição de segurança conforme catálogo ISO 20022 da B3 (ex: 8 = B3).
    #[schema(example = "8")]
    pub security_source: Option<String>,

    /// Código de identificação do mercado (MIC - Market Identifier Code, ex: BVMF).
    #[schema(example = "BVMF")]
    pub market_identifier_code: Option<String>,

    /// Código base do ativo objeto (ex: PETR, VALE, BOVA).
    #[schema(example = "PETR")]
    pub asset: Option<String>,

    /// Descrição oficial em português da classe do ativo emitida pela B3.
    #[schema(example = "PETROLEO BRASILEIRO S.A. PETROBRAS")]
    pub asset_description: Option<String>,

    /// Código do tipo de mercado de negociação (ex: EQ = Ações, OP = Opções, FU = Futuros).
    #[schema(example = "EQ")]
    pub market_code: Option<String>,

    /// Nome descritivo do mercado de negociação na B3.
    #[schema(example = "VISTA")]
    pub market_name: Option<String>,

    /// Código do segmento operacional do mercado (ex: CASH = Mercado à Vista).
    #[schema(example = "CASH")]
    pub segment_code: Option<String>,

    /// Nome descritivo do segmento de negociação.
    #[schema(example = "MERCADO A VISTA")]
    pub segment_name: Option<String>,

    /// Tipo cadastral do instrumento (ex: SHARES, OPTIONS, FUTURES).
    #[schema(example = "SHARES")]
    pub instrument_type: Option<String>,

    /// Categoria de segurança (ex: PREFERRED_SHARES, COMMON_SHARES).
    #[schema(example = "PREFERRED_SHARES")]
    pub security_category: Option<String>,

    /// Código CFI (Classification of Financial Instruments - ISO 10962, ex: EPNTFR).
    #[schema(example = "EPNTFR")]
    pub cfi_code: Option<String>,

    /// Status operacional do instrumento (ex: Active, Expired).
    #[schema(example = "Active")]
    pub instrument_status: Option<String>,

    /// Data de vencimento do contrato (aplicável a derivativos, opções, futuros e renda fixa).
    #[schema(example = "2027-01-02")]
    pub maturity_date: Option<NaiveDate>,

    /// Símbolo de negociação (ticker) vigente para o ativo (ex: PETR4).
    #[schema(example = "PETR4")]
    pub ticker_symbol: Option<String>,

    /// Código ISIN internacional associado (ex: BRPETRACNPR6).
    #[schema(example = "BRPETRACNPR6")]
    pub isin: Option<String>,

    /// Proveniência e rastreabilidade do arquivo oficial B3 que originou os dados.
    pub provenance: Provenance,

    /// Metadados operacionais internos exibidos condicionalmente via `include_internal=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

/// Identificador de instrumento com janela de validade temporal (Ticker, ISIN, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IdentifierItem {
    /// Identificador único interno (UUID) do registro.
    #[schema(example = "c12d8a54-7123-4876-90ab-123456789abc")]
    pub id: String,

    /// ID do instrumento pai ao qual este identificador pertence.
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub instrument_id: String,

    /// Tipo de identificador (ex: `ticker`, `isin`, `security_id`).
    #[schema(example = "ticker")]
    pub identifier_type: String,

    /// Valor do código identificador (ex: `PETR4`, `BRPETRACNPR6`).
    #[schema(example = "PETR4")]
    pub identifier_value: String,

    /// Mercado associado ao identificador quando aplicável.
    #[schema(example = "EQ")]
    pub market_code: Option<String>,

    /// Data de início da vigência do identificador para este instrumento.
    #[schema(example = "2010-01-01")]
    pub valid_from: Option<NaiveDate>,

    /// Data final da vigência (nula se ainda for o identificador vigente atual).
    #[schema(example = "2026-12-31")]
    pub valid_to: Option<NaiveDate>,

    /// Timestamp UTC de criação do registro no banco.
    #[schema(example = "2026-03-01T22:00:00Z")]
    pub created_at: DateTime<Utc>,
}

/// Parâmetros de filtro para listagem de instrumentos.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct InstrumentFilterParams {
    /// Filtro por símbolo de negociação (ticker, case-insensitive, ex: `PETR4`).
    #[param(example = "PETR4")]
    pub ticker: Option<String>,

    /// Filtro por código ISIN de 12 caracteres (ex: `BRPETRACNPR6`).
    #[param(example = "BRPETRACNPR6")]
    pub isin: Option<String>,

    /// Filtro por Security ID oficial da B3 (ex: `PETR4`).
    #[param(example = "PETR4")]
    pub security_id: Option<String>,

    /// Filtro por código de mercado (ex: `EQ`, `OP`, `FU`).
    #[param(example = "EQ")]
    pub market_code: Option<String>,

    /// Filtro por segmento de negociação (ex: `CASH`, `DERIVATIVES`).
    #[param(example = "CASH")]
    pub segment_code: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/instruments",
    tag = "B3 Instruments File (BVBG.028.02)",
    summary = "Listagem de Instrumentos Financeiros (BVBG.028.02)",
    description = "Retorna lista paginada de instrumentos financeiros cadastrados oficialmente na B3 através do arquivo BVBG.028.02 (InstrumentsConsolidated). \n\n### Regras de Negócio e Proveniência:\n- A raiz de instrumentos B3 é a âncora cadastral primária (`security_id`).\n- Suporta filtros por ticker, ISIN, código de mercado e segmento.\n- Ordenação determinística por `security_id` ascendente com tie-breaker único.\n- Metadados de proveniência (`provenance`) acompanham cada registro, incluindo SHA-256 e arquivo de origem.\n- O parâmetro `include_internal=true` expõe timestamps granulares de alteração por campo (`field_changed_at`).",
    params(PaginationParams, InstrumentFilterParams),
    responses(
        (status = 200, description = "Lista paginada de instrumentos recuperada com sucesso.", body = PaginatedResponse<InstrumentItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido no cabeçalho Authorization."),
        (status = 500, description = "Erro interno ao processar a consulta no banco de dados PostgreSQL.")
    )
)]
pub async fn list_instruments(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<InstrumentFilterParams>,
) -> Result<Json<PaginatedResponse<InstrumentItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let isin = filters.isin.as_deref().map(|s| s.trim().to_uppercase());
    let sec_id = filters.security_id.as_deref().map(|s| s.trim());
    let market = filters
        .market_code
        .as_deref()
        .map(|s| s.trim().to_uppercase());
    let segment = filters
        .segment_code
        .as_deref()
        .map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(DISTINCT i.id)
        FROM b3_instruments i
        LEFT JOIN b3_instrument_identifiers ident ON ident.instrument_id = i.id
        WHERE ($1::varchar IS NULL OR ident.identifier_type = 'ticker' AND ident.identifier_value = $1)
          AND ($2::varchar IS NULL OR ident.identifier_type = 'isin' AND ident.identifier_value = $2)
          AND ($3::varchar IS NULL OR i.security_id = $3)
          AND ($4::varchar IS NULL OR i.market_code = $4)
          AND ($5::varchar IS NULL OR i.segment_code = $5)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&ticker)
        .bind(&isin)
        .bind(sec_id)
        .bind(&market)
        .bind(&segment)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_isin
        FROM b3_instruments i
        JOIN b3_source_files f ON f.id = i.source_file_id
        LEFT JOIN b3_instrument_identifiers ident ON ident.instrument_id = i.id
        WHERE ($1::varchar IS NULL OR ident.identifier_type = 'ticker' AND ident.identifier_value = $1)
          AND ($2::varchar IS NULL OR ident.identifier_type = 'isin' AND ident.identifier_value = $2)
          AND ($3::varchar IS NULL OR i.security_id = $3)
          AND ($4::varchar IS NULL OR i.market_code = $4)
          AND ($5::varchar IS NULL OR i.segment_code = $5)
        GROUP BY i.id, f.id
        ORDER BY i.security_id ASC
        LIMIT $6 OFFSET $7
    "#;

    let rows = sqlx::query(select_query)
        .bind(&ticker)
        .bind(&isin)
        .bind(sec_id)
        .bind(&market)
        .bind(&segment)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|row| {
            let id: String = row.get("id");
            let sec_id: String = row.get("security_id");
            let provenance = Provenance {
                source_file_type: row.get("source_file_type"),
                source_file_id: row.get("source_file_id"),
                source_file_name: row.get("original_file_name"),
                source_member_name: row.get("source_member_name"),
                reference_date: row.get("reference_date"),
                sha256: row.get("sha256"),
                ingested_at: row.get("ingested_at"),
            };

            let internal_metadata = if pagination.include_internal() {
                Some(InternalMetadata {
                    data_changed_at: row.get("data_changed_at"),
                    field_changed_at: row.get("field_changed_at"),
                    resolution_status: Some("resolved".to_string()),
                    last_seen_at: row.get("last_seen_at"),
                    source_line_number: row.get("source_line_number"),
                })
            } else {
                None
            };

            InstrumentItem {
                id,
                security_id: sec_id,
                security_source: row.get("security_source"),
                market_identifier_code: row.get("market_identifier_code"),
                asset: row.get("asset"),
                asset_description: row.get("asset_description"),
                market_code: row.get("market_code"),
                market_name: row.get("market_name"),
                segment_code: row.get("segment_code"),
                segment_name: row.get("segment_name"),
                instrument_type: row.get("instrument_type"),
                security_category: row.get("security_category"),
                cfi_code: row.get("cfi_code"),
                instrument_status: row.get("instrument_status"),
                maturity_date: row.get("maturity_date"),
                ticker_symbol: row.get("current_ticker"),
                isin: row.get("current_isin"),
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
    path = "/api/v1/b3/instruments/{id}",
    tag = "B3 Instruments File (BVBG.028.02)",
    summary = "Obter Instrumento por UUID ou Security ID",
    description = "Recupera o cadastro completo de um instrumento financeiro específico através do seu identificador único interno (UUID) ou pelo `security_id` oficial da B3.",
    params(
        ("id" = String, Path, description = "UUID interno ou security_id do instrumento (ex.: `PETR4` ou `550e8400-e29b-41d4-a716-446655440000`)", example = "PETR4")
    ),
    responses(
        (status = 200, description = "Instrumento localizado com sucesso.", body = InstrumentItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Instrumento não encontrado para o ID especificado.")
    )
)]
pub async fn get_instrument(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<InstrumentItem>, (StatusCode, String)> {
    let query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_isin
        FROM b3_instruments i
        JOIN b3_source_files f ON f.id = i.source_file_id
        WHERE i.id = $1 OR i.security_id = $1
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Instrument not found".to_string()))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: row.get("reference_date"),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(InstrumentItem {
        id: row.get("id"),
        security_id: row.get("security_id"),
        security_source: row.get("security_source"),
        market_identifier_code: row.get("market_identifier_code"),
        asset: row.get("asset"),
        asset_description: row.get("asset_description"),
        market_code: row.get("market_code"),
        market_name: row.get("market_name"),
        segment_code: row.get("segment_code"),
        segment_name: row.get("segment_name"),
        instrument_type: row.get("instrument_type"),
        security_category: row.get("security_category"),
        cfi_code: row.get("cfi_code"),
        instrument_status: row.get("instrument_status"),
        maturity_date: row.get("maturity_date"),
        ticker_symbol: row.get("current_ticker"),
        isin: row.get("current_isin"),
        provenance,
        internal_metadata: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/instruments/by-ticker/{ticker_symbol}",
    tag = "B3 Instruments File (BVBG.028.02)",
    summary = "Obter Instrumento por Ticker",
    description = "Busca o instrumento atualmente ativo associado ao símbolo de negociação (ticker). Respeita janelas de validade temporal (`valid_from` e `valid_to`), retornando o registro de maior vigência recente.",
    params(
        ("ticker_symbol" = String, Path, description = "Símbolo de negociação da B3 (ex.: `PETR4`, `VALE3`)", example = "PETR4")
    ),
    responses(
        (status = 200, description = "Instrumento localizado com sucesso.", body = InstrumentItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Instrumento não encontrado para o ticker informado.")
    )
)]
pub async fn get_instrument_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> Result<Json<InstrumentItem>, (StatusCode, String)> {
    let clean_ticker = ticker.trim().to_uppercase();
    let query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            ident.identifier_value as current_ticker,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'isin' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_isin
        FROM b3_instrument_identifiers ident
        JOIN b3_instruments i ON i.id = ident.instrument_id
        JOIN b3_source_files f ON f.id = i.source_file_id
        WHERE ident.identifier_type = 'ticker' AND ident.identifier_value = $1
        ORDER BY ident.valid_to NULLS FIRST, ident.valid_from DESC
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(&clean_ticker)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Instrument not found for ticker {clean_ticker}"),
        ))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: row.get("reference_date"),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(InstrumentItem {
        id: row.get("id"),
        security_id: row.get("security_id"),
        security_source: row.get("security_source"),
        market_identifier_code: row.get("market_identifier_code"),
        asset: row.get("asset"),
        asset_description: row.get("asset_description"),
        market_code: row.get("market_code"),
        market_name: row.get("market_name"),
        segment_code: row.get("segment_code"),
        segment_name: row.get("segment_name"),
        instrument_type: row.get("instrument_type"),
        security_category: row.get("security_category"),
        cfi_code: row.get("cfi_code"),
        instrument_status: row.get("instrument_status"),
        maturity_date: row.get("maturity_date"),
        ticker_symbol: row.get("current_ticker"),
        isin: row.get("current_isin"),
        provenance,
        internal_metadata: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/instruments/by-isin/{isin}",
    tag = "B3 Instruments File (BVBG.028.02)",
    summary = "Obter Instrumento por Código ISIN",
    description = "Busca o instrumento oficial correspondente ao código ISIN internacional (ISO 6166 de 12 caracteres).",
    params(
        ("isin" = String, Path, description = "Código ISIN de 12 caracteres (ex.: `BRPETRACNPR6`)", example = "BRPETRACNPR6")
    ),
    responses(
        (status = 200, description = "Instrumento localizado com sucesso.", body = InstrumentItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Instrumento não encontrado para o ISIN informado.")
    )
)]
pub async fn get_instrument_by_isin(
    State(state): State<Arc<AppState>>,
    Path(isin): Path<String>,
) -> Result<Json<InstrumentItem>, (StatusCode, String)> {
    let clean_isin = isin.trim().to_uppercase();
    let query = r#"
        SELECT 
            i.id, i.security_id, i.security_source, i.market_identifier_code,
            i.asset, i.asset_description, i.market_code, i.market_name,
            i.segment_code, i.segment_name, i.instrument_type, i.security_category,
            i.cfi_code, i.instrument_status, i.maturity_date,
            i.source_file_id, i.source_member_name, i.source_line_number,
            i.data_changed_at, i.field_changed_at, i.last_seen_at,
            f.source_file_type, f.original_file_name, f.reference_date, f.sha256, f.ingested_at,
            (SELECT identifier_value FROM b3_instrument_identifiers WHERE instrument_id = i.id AND identifier_type = 'ticker' ORDER BY valid_to NULLS FIRST, valid_from DESC LIMIT 1) as current_ticker,
            ident.identifier_value as current_isin
        FROM b3_instrument_identifiers ident
        JOIN b3_instruments i ON i.id = ident.instrument_id
        JOIN b3_source_files f ON f.id = i.source_file_id
        WHERE ident.identifier_type = 'isin' AND ident.identifier_value = $1
        ORDER BY ident.valid_to NULLS FIRST, ident.valid_from DESC
        LIMIT 1
    "#;

    let row = sqlx::query(query)
        .bind(&clean_isin)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            format!("Instrument not found for ISIN {clean_isin}"),
        ))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: row.get("reference_date"),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(InstrumentItem {
        id: row.get("id"),
        security_id: row.get("security_id"),
        security_source: row.get("security_source"),
        market_identifier_code: row.get("market_identifier_code"),
        asset: row.get("asset"),
        asset_description: row.get("asset_description"),
        market_code: row.get("market_code"),
        market_name: row.get("market_name"),
        segment_code: row.get("segment_code"),
        segment_name: row.get("segment_name"),
        instrument_type: row.get("instrument_type"),
        security_category: row.get("security_category"),
        cfi_code: row.get("cfi_code"),
        instrument_status: row.get("instrument_status"),
        maturity_date: row.get("maturity_date"),
        ticker_symbol: row.get("current_ticker"),
        isin: row.get("current_isin"),
        provenance,
        internal_metadata: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/instruments/{id}/identifiers",
    tag = "B3 Instruments File (BVBG.028.02)",
    summary = "Histórico de Identificadores e Janelas Temporais",
    description = "Retorna todos os identificadores conhecidos (tickers, ISINs, códigos anteriores) vinculados ao instrumento, com suas respectivas janelas de validade temporal (`valid_from` até `valid_to`).",
    params(
        ("id" = String, Path, description = "UUID interno ou security_id do instrumento", example = "PETR4")
    ),
    responses(
        (status = 200, description = "Lista de identificadores retornada com sucesso.", body = Vec<IdentifierItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn list_instrument_identifiers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<IdentifierItem>>, (StatusCode, String)> {
    let query = r#"
        SELECT id, instrument_id, identifier_type, identifier_value, market_code, valid_from, valid_to, created_at
        FROM b3_instrument_identifiers
        WHERE instrument_id = $1 OR instrument_id = (SELECT id FROM b3_instruments WHERE security_id = $1 LIMIT 1)
        ORDER BY valid_from DESC NULLS LAST, identifier_type ASC
    "#;

    let rows = sqlx::query(query)
        .bind(&id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| IdentifierItem {
            id: r.get("id"),
            instrument_id: r.get("instrument_id"),
            identifier_type: r.get("identifier_type"),
            identifier_value: r.get("identifier_value"),
            market_code: r.get("market_code"),
            valid_from: r.get("valid_from"),
            valid_to: r.get("valid_to"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(items))
}
