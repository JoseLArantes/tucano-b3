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

/// Cotação oficial de fechamento diário do boletim PriceReport (BVBG.086.01).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceReportQuoteItem {
    /// Identificador único interno (UUID) da cotação.
    #[schema(example = "41b2c3d4-5e6f-7081-92a3-b4c5d6e7f809")]
    pub id: String,

    /// UUID do instrumento pai na tabela `b3_instruments` (quando resolvido com sucesso).
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub instrument_id: Option<String>,

    /// Data do pregão em que os preços foram praticados (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub trade_date: NaiveDate,

    /// Identificador oficial do instrumento na estrutura de negociação da B3.
    #[schema(example = "PETR4")]
    pub security_id: Option<String>,

    /// Símbolo de negociação (ticker) no pregão.
    #[schema(example = "PETR4")]
    pub ticker_symbol: Option<String>,

    /// Código de identificação do mercado (MIC, ex.: BVMF).
    #[schema(example = "BVMF")]
    pub market_identifier_code: Option<String>,

    /// Prazo em dias até a liquidação (chave discriminadora para contratos a termo).
    #[schema(example = 30)]
    pub days_to_settlement: Option<i32>,

    /// Quantidade física total de títulos negociados no pregão.
    #[schema(example = "15200000")]
    pub trade_quantity: Option<Decimal>,

    /// Volume financeiro total negociado na moeda BRL (Reais).
    #[schema(example = "547200000.00")]
    pub financial_volume_brl: Option<Decimal>,

    /// Volume financeiro total convertido em USD quando aplicável.
    #[schema(example = "105230769.23")]
    pub financial_volume_usd: Option<Decimal>,

    /// Posições em aberto no fechamento da sessão (aplicável a derivativos e opções).
    #[schema(example = "45000")]
    pub open_interest: Option<Decimal>,

    /// Melhor preço de compra registrado no livro de ofertas no fechamento.
    #[schema(example = "35.98")]
    pub best_bid_price: Option<Decimal>,

    /// Melhor preço de venda registrado no livro de ofertas no fechamento.
    #[schema(example = "36.00")]
    pub best_ask_price: Option<Decimal>,

    /// Preço de abertura praticado no primeiro negócio da sessão.
    #[schema(example = "35.50")]
    pub first_price: Option<Decimal>,

    /// Menor preço de negócio registrado ao longo do pregão.
    #[schema(example = "35.20")]
    pub minimum_price: Option<Decimal>,

    /// Preço médio ponderado pelo volume de negócios da sessão.
    #[schema(example = "35.85")]
    pub average_price: Option<Decimal>,

    /// Maior preço de negócio registrado ao longo do pregão.
    #[schema(example = "36.40")]
    pub maximum_price: Option<Decimal>,

    /// Preço de fechamento praticado no último negócio da sessão.
    #[schema(example = "36.00")]
    pub last_price: Option<Decimal>,

    /// Quantidade de transações regulares de compra e venda realizadas na sessão.
    #[schema(example = 45200)]
    pub regular_transactions_quantity: Option<i32>,

    /// Status da amarração cadastral com a tabela de instrumentos (`resolved`, `unresolved`, `ambiguous`).
    #[schema(example = "resolved")]
    pub resolution_status: String,

    /// Metadados de proveniência e integridade do arquivo oficial B3.
    pub provenance: Provenance,

    /// Metadados operacionais internos exibidos condicionalmente via `include_internal=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

/// Parâmetros de filtro para cotações do PriceReport (BVBG.086).
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PriceReportFilterParams {
    /// Data da sessão de negociação a consultar (YYYY-MM-DD).
    #[param(example = "2026-03-01")]
    pub trade_date: Option<NaiveDate>,

    /// Símbolo de negociação (ticker, case-insensitive, ex: `PETR4`).
    #[param(example = "PETR4")]
    pub ticker: Option<String>,

    /// Security ID oficial do instrumento na B3 (ex: `PETR4`).
    #[param(example = "PETR4")]
    pub security_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/price-report/quotes",
    tag = "B3 PriceReport (BVBG.086.01)",
    summary = "Listagem de Cotações do PriceReport (BVBG.086.01)",
    description = "Retorna cotações oficiais diárias completas emitidas pelo arquivo PriceReport (BVBG.086.01) da B3. \n\n### Regras e Características:\n- Abrange ações, opções, contratos futuros e termos.\n- O campo `days_to_settlement` faz parte da chave de unicidade para contratos a termo.\n- Contratos mantidos em isolamento estrito sem fusão com o Simplified Price Report (`BVBG.186.01`).\n- Preços decimais são preservados com precisão exata (`rust_decimal`) e serializados como strings canônicas sem separador de milhar.",
    params(PaginationParams, PriceReportFilterParams),
    responses(
        (status = 200, description = "Cotações do PriceReport retornadas com sucesso.", body = PaginatedResponse<PriceReportQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 500, description = "Erro interno ao consultar o banco de dados.")
    )
)]
pub async fn list_price_report_quotes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<PriceReportFilterParams>,
) -> Result<Json<PaginatedResponse<PriceReportQuoteItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let sec_id = filters.security_id.as_deref().map(|s| s.trim());

    let count_query = r#"
        SELECT count(*)
        FROM b3_price_report_quotes q
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.security_id = $3)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(sec_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.days_to_settlement, q.trade_quantity,
            q.financial_volume_brl, q.financial_volume_usd, q.open_interest,
            q.best_bid_price, q.best_ask_price, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.regular_transactions_quantity,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_price_report_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.security_id = $3)
        ORDER BY q.trade_date DESC, q.ticker_symbol ASC
        LIMIT $4 OFFSET $5
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(sec_id)
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

            PriceReportQuoteItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                trade_date: row.get("trade_date"),
                security_id: row.get("security_id"),
                ticker_symbol: row.get("ticker_symbol"),
                market_identifier_code: row.get("market_identifier_code"),
                days_to_settlement: row.get("days_to_settlement"),
                trade_quantity: row.get("trade_quantity"),
                financial_volume_brl: row.get("financial_volume_brl"),
                financial_volume_usd: row.get("financial_volume_usd"),
                open_interest: row.get("open_interest"),
                best_bid_price: row.get("best_bid_price"),
                best_ask_price: row.get("best_ask_price"),
                first_price: row.get("first_price"),
                minimum_price: row.get("minimum_price"),
                average_price: row.get("average_price"),
                maximum_price: row.get("maximum_price"),
                last_price: row.get("last_price"),
                regular_transactions_quantity: row.get("regular_transactions_quantity"),
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
    path = "/api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}",
    tag = "B3 PriceReport (BVBG.086.01)",
    summary = "Série Histórica do PriceReport por Ticker",
    description = "Recupera a série temporal de cotações diárias do arquivo PriceReport para um símbolo de negociação (ticker).",
    params(
        ("ticker_symbol" = String, Path, description = "Símbolo de negociação da B3 (ex: PETR4)", example = "PETR4"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Série temporal de cotações recuperada com sucesso.", body = PaginatedResponse<PriceReportQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_price_report_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PriceReportQuoteItem>>, (StatusCode, String)> {
    let filters = PriceReportFilterParams {
        trade_date: None,
        ticker: Some(ticker),
        security_id: None,
    };
    list_price_report_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/price-report/quotes/by-date/{trade_date}",
    tag = "B3 PriceReport (BVBG.086.01)",
    summary = "Cotações do PriceReport por Data de Pregão",
    description = "Retorna todos os registros de cotação do PriceReport praticados em uma data específica de pregão da B3.",
    params(
        ("trade_date" = NaiveDate, Path, description = "Data da sessão de negociação (YYYY-MM-DD)", example = "2026-03-01"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Cotações da sessão retornadas com sucesso.", body = PaginatedResponse<PriceReportQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_price_report_by_date(
    State(state): State<Arc<AppState>>,
    Path(date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<PriceReportQuoteItem>>, (StatusCode, String)> {
    let filters = PriceReportFilterParams {
        trade_date: Some(date),
        ticker: None,
        security_id: None,
    };
    list_price_report_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/price-report/quotes/{quote_id}",
    tag = "B3 PriceReport (BVBG.086.01)",
    summary = "Obter Cotação do PriceReport por UUID",
    description = "Recupera os detalhes de um registro específico de cotação através do seu identificador único (UUID).",
    params(
        ("quote_id" = String, Path, description = "UUID da cotação", example = "41b2c3d4-5e6f-7081-92a3-b4c5d6e7f809")
    ),
    responses(
        (status = 200, description = "Cotação localizada com sucesso.", body = PriceReportQuoteItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Cotação não encontrada para o ID informado.")
    )
)]
pub async fn get_price_report_by_id(
    State(state): State<Arc<AppState>>,
    Path(quote_id): Path<String>,
) -> Result<Json<PriceReportQuoteItem>, (StatusCode, String)> {
    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.security_id, q.ticker_symbol,
            q.market_identifier_code, q.days_to_settlement, q.trade_quantity,
            q.financial_volume_brl, q.financial_volume_usd, q.open_interest,
            q.best_bid_price, q.best_ask_price, q.first_price, q.minimum_price,
            q.average_price, q.maximum_price, q.last_price, q.regular_transactions_quantity,
            q.resolution_status, q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_price_report_quotes q
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
            "Price report quote not found".to_string(),
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

    Ok(Json(PriceReportQuoteItem {
        id: row.get("id"),
        instrument_id: row.get("instrument_id"),
        trade_date: row.get("trade_date"),
        security_id: row.get("security_id"),
        ticker_symbol: row.get("ticker_symbol"),
        market_identifier_code: row.get("market_identifier_code"),
        days_to_settlement: row.get("days_to_settlement"),
        trade_quantity: row.get("trade_quantity"),
        financial_volume_brl: row.get("financial_volume_brl"),
        financial_volume_usd: row.get("financial_volume_usd"),
        open_interest: row.get("open_interest"),
        best_bid_price: row.get("best_bid_price"),
        best_ask_price: row.get("best_ask_price"),
        first_price: row.get("first_price"),
        minimum_price: row.get("minimum_price"),
        average_price: row.get("average_price"),
        maximum_price: row.get("maximum_price"),
        last_price: row.get("last_price"),
        regular_transactions_quantity: row.get("regular_transactions_quantity"),
        resolution_status: row.get("resolution_status"),
        provenance,
        internal_metadata: None,
    }))
}
