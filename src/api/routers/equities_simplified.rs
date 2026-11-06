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

/// Cotação oficial diária do boletim Simplified Price Report para o mercado de ações (BVBG.186.01).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EquitiesSimplifiedQuoteItem {
    /// Identificador único interno (UUID) da cotação.
    #[schema(example = "82c3d4e5-f6a7-8091-b2c3-d4e5f6a7b8c9")]
    pub id: String,

    /// UUID do instrumento pai na tabela `b3_instruments` (quando resolvido com sucesso).
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub instrument_id: Option<String>,

    /// Data do pregão em que os preços foram praticados (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub trade_date: NaiveDate,

    /// Identificador oficial do ativo na B3.
    #[schema(example = "PETR4")]
    pub security_id: Option<String>,

    /// Símbolo de negociação da ação (ticker).
    #[schema(example = "PETR4")]
    pub ticker_symbol: Option<String>,

    /// Código de identificação do mercado (MIC, ex: BVMF).
    #[schema(example = "BVMF")]
    pub market_identifier_code: Option<String>,

    /// Preço de abertura da ação no primeiro negócio da sessão.
    #[schema(example = "35.50")]
    pub first_price: Option<Decimal>,

    /// Menor preço registrado para a ação na sessão.
    #[schema(example = "35.20")]
    pub minimum_price: Option<Decimal>,

    /// Preço médio ponderado da ação no pregão.
    #[schema(example = "35.85")]
    pub average_price: Option<Decimal>,

    /// Maior preço registrado para a ação na sessão.
    #[schema(example = "36.40")]
    pub maximum_price: Option<Decimal>,

    /// Preço de fechamento da ação no último negócio da sessão.
    #[schema(example = "36.00")]
    pub last_price: Option<Decimal>,

    /// Volume financeiro total negociado em moeda corrente (BRL).
    #[schema(example = "547200000.00")]
    pub financial_volume_brl: Option<Decimal>,

    /// Status da amarração cadastral com a tabela de instrumentos (`resolved`, `unresolved`, `ambiguous`).
    #[schema(example = "resolved")]
    pub resolution_status: String,

    /// Metadados de proveniência do arquivo oficial B3.
    pub provenance: Provenance,

    /// Metadados operacionais internos exibidos quando `include_internal=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

/// Comparativo cruzado de preços de fechamento entre o PriceReport completo (BVBG.086) e o Simplificado de Ações (BVBG.186).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceComparisonItem {
    /// Símbolo de negociação (ticker) comparado.
    #[schema(example = "PETR4")]
    pub ticker_symbol: String,

    /// Data do pregão comparado (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub trade_date: NaiveDate,

    /// Preço de fechamento publicado no boletim completo BVBG.086.01.
    #[schema(example = "36.00")]
    pub bvbg086_last_price: Option<Decimal>,

    /// Preço de fechamento publicado no boletim simplificado BVBG.186.01.
    #[schema(example = "36.00")]
    pub bvbg186_last_price: Option<Decimal>,

    /// Diferença absoluta (`bvbg086_last_price - bvbg186_last_price`).
    #[schema(example = "0.00")]
    pub diff: Option<Decimal>,

    /// Verdadeiro (`true`) se ambos os boletins coincidem exatamente no valor de fechamento.
    #[schema(example = true)]
    pub matches: bool,
}

/// Parâmetros de filtro para cotações simplificadas de ações (BVBG.186).
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EquitiesSimplifiedFilterParams {
    /// Data da sessão de negociação (YYYY-MM-DD).
    #[param(example = "2026-03-01")]
    pub trade_date: Option<NaiveDate>,

    /// Símbolo de negociação da ação (ticker, case-insensitive, ex: `PETR4`).
    #[param(example = "PETR4")]
    pub ticker: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/equities-simplified-price-report/quotes",
    tag = "B3 Simplified Price Report - Equities (BVBG.186.01)",
    summary = "Listagem do Relatório Simplificado de Ações (BVBG.186.01)",
    description = "Retorna cotações de ações do arquivo Simplified Price Report (BVBG.186.01) da B3. \n\n### Regras:\n- Focado exclusivamente no segmento de ações à vista.\n- Mantido em separação explícita do BVBG.086.01 para evitar sobreposição ou perda de linhagem.\n- Permite verificar rapidamente abertura, máxima, mínima, média, fechamento e volume BRL.",
    params(PaginationParams, EquitiesSimplifiedFilterParams),
    responses(
        (status = 200, description = "Cotações simplificadas retornadas com sucesso.", body = PaginatedResponse<EquitiesSimplifiedQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 500, description = "Erro interno ao consultar o banco de dados.")
    )
)]
pub async fn list_equities_simplified_quotes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<EquitiesSimplifiedFilterParams>,
) -> Result<Json<PaginatedResponse<EquitiesSimplifiedQuoteItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());

    let count_query = r#"
        SELECT count(*)
        FROM b3_equities_simplified_price_quotes q
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.financial_volume_brl,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_equities_simplified_price_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
        ORDER BY q.trade_date DESC, q.ticker_symbol ASC
        LIMIT $3 OFFSET $4
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
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
                reference_date: Some(row.get("trade_date")),
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

            EquitiesSimplifiedQuoteItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                trade_date: row.get("trade_date"),
                security_id: row.get("security_id"),
                ticker_symbol: row.get("ticker_symbol"),
                market_identifier_code: row.get("market_identifier_code"),
                first_price: row.get("first_price"),
                minimum_price: row.get("minimum_price"),
                average_price: row.get("average_price"),
                maximum_price: row.get("maximum_price"),
                last_price: row.get("last_price"),
                financial_volume_brl: row.get("financial_volume_brl"),
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
    path = "/api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}",
    tag = "B3 Simplified Price Report - Equities (BVBG.186.01)",
    summary = "Série do Relatório Simplificado por Ticker",
    description = "Recupera o histórico de cotações simplificadas de ações para um ticker ao longo das sessões.",
    params(
        ("ticker_symbol" = String, Path, description = "Símbolo de negociação da ação (ex: PETR4)", example = "PETR4"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Série temporal retornada com sucesso.", body = PaginatedResponse<EquitiesSimplifiedQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_equities_simplified_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<EquitiesSimplifiedQuoteItem>>, (StatusCode, String)> {
    let filters = EquitiesSimplifiedFilterParams {
        trade_date: None,
        ticker: Some(ticker),
    };
    list_equities_simplified_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}",
    tag = "B3 Simplified Price Report - Equities (BVBG.186.01)",
    summary = "Cotações Simplificadas por Data de Pregão",
    description = "Retorna todas as cotações simplificadas de ações negociadas em uma data específica de pregão da B3.",
    params(
        ("trade_date" = NaiveDate, Path, description = "Data da sessão de negociação (YYYY-MM-DD)", example = "2026-03-01"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Cotações da sessão retornadas com sucesso.", body = PaginatedResponse<EquitiesSimplifiedQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_equities_simplified_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<EquitiesSimplifiedQuoteItem>>, (StatusCode, String)> {
    let filters = EquitiesSimplifiedFilterParams {
        trade_date: Some(date),
        ticker: None,
    };
    list_equities_simplified_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/equities-simplified-price-report/quotes/{quote_id}",
    tag = "B3 Simplified Price Report - Equities (BVBG.186.01)",
    summary = "Obter Cotação Simplificada por UUID",
    description = "Recupera o registro específico de cotação simplificada através do seu identificador único (UUID).",
    params(
        ("quote_id" = String, Path, description = "UUID da cotação", example = "82c3d4e5-f6a7-8091-b2c3-d4e5f6a7b8c9")
    ),
    responses(
        (status = 200, description = "Cotação localizada com sucesso.", body = EquitiesSimplifiedQuoteItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Cotação não encontrada para o ID informado.")
    )
)]
pub async fn get_equities_simplified_by_id(
    State(state): State<Arc<AppState>>,
    Path(quote_id): Path<String>,
) -> Result<Json<EquitiesSimplifiedQuoteItem>, (StatusCode, String)> {
    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.financial_volume_brl,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_equities_simplified_price_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE q.id = $1
        LIMIT 1
    "#;

    let row = sqlx::query(select_query)
        .bind(&quote_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Simplified quote not found".to_string(),
        ))?;

    let provenance = Provenance {
        source_file_type: row.get("source_file_type"),
        source_file_id: row.get("source_file_id"),
        source_file_name: row.get("original_file_name"),
        source_member_name: row.get("source_member_name"),
        reference_date: Some(row.get("trade_date")),
        sha256: row.get("sha256"),
        ingested_at: row.get("ingested_at"),
    };

    Ok(Json(EquitiesSimplifiedQuoteItem {
        id: row.get("id"),
        instrument_id: row.get("instrument_id"),
        trade_date: row.get("trade_date"),
        security_id: row.get("security_id"),
        ticker_symbol: row.get("ticker_symbol"),
        market_identifier_code: row.get("market_identifier_code"),
        first_price: row.get("first_price"),
        minimum_price: row.get("minimum_price"),
        average_price: row.get("average_price"),
        maximum_price: row.get("maximum_price"),
        last_price: row.get("last_price"),
        financial_volume_brl: row.get("financial_volume_brl"),
        resolution_status: row.get("resolution_status"),
        provenance,
        internal_metadata: None,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/price-report-comparison/{trade_date}",
    tag = "B3 Simplified Price Report - Equities (BVBG.186.01)",
    summary = "Comparativo Cruzado de Preços (BVBG.086 vs BVBG.186)",
    description = "Executa auditoria de consistência comparando os preços de fechamento publicados em `BVBG.086.01` contra os publicados em `BVBG.186.01` na mesma data de pregão. \n\n### Propósito:\n- Garante que a simplificação de mercado de ações não diverge dos preços globais do PriceReport.\n- Expõe a diferença matemática (`diff`) e a flag booleana de concordância exata (`matches`).",
    params(
        ("trade_date" = NaiveDate, Path, description = "Data da sessão de negociação a comparar (YYYY-MM-DD)", example = "2026-03-01")
    ),
    responses(
        (status = 200, description = "Comparativo de preços retornado com sucesso.", body = Vec<PriceComparisonItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 500, description = "Erro interno ao processar a comparação.")
    )
)]
pub async fn compare_price_reports(
    State(state): State<Arc<AppState>>,
    Path(trade_date): Path<NaiveDate>,
) -> Result<Json<Vec<PriceComparisonItem>>, (StatusCode, String)> {
    let query = r#"
        SELECT 
            COALESCE(p.ticker_symbol, s.ticker_symbol) as ticker,
            p.trade_date,
            p.last_price as p_last,
            s.last_price as s_last
        FROM b3_price_report_quotes p
        FULL OUTER JOIN b3_equities_simplified_price_quotes s 
            ON s.trade_date = p.trade_date AND s.ticker_symbol = p.ticker_symbol
        WHERE (p.trade_date = $1 OR s.trade_date = $1)
        ORDER BY ticker ASC
    "#;

    let rows = sqlx::query(query)
        .bind(trade_date)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = rows
        .into_iter()
        .map(|r| {
            let p_last: Option<Decimal> = r.get("p_last");
            let s_last: Option<Decimal> = r.get("s_last");
            let diff = match (p_last, s_last) {
                (Some(p), Some(s)) => Some(p - s),
                _ => None,
            };
            let matches = match diff {
                Some(d) => d.is_zero(),
                None => false,
            };

            PriceComparisonItem {
                ticker_symbol: r.get("ticker"),
                trade_date,
                bvbg086_last_price: p_last,
                bvbg186_last_price: s_last,
                diff,
                matches,
            }
        })
        .collect();

    Ok(Json(items))
}
