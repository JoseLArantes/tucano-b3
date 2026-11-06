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

pub const COTAHIST_DISCLAIMER: &str = "AVISO: Preços brutos nominais de negociação histórica da B3. Não ajustados retroativamente por proventos em dinheiro (dividendos/JCP), desdobramentos (splits), grupamentos ou bonificações corporativas.";

/// Cotação histórica bruta da série oficial COTAHIST da B3 (arquivo de largura fixa de 245 bytes).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CotahistQuoteItem {
    /// Identificador único interno (UUID) do registro.
    #[schema(example = "90a1b2c3-d4e5-6789-0123-456789abcdef")]
    pub id: String,

    /// UUID do instrumento pai na tabela `b3_instruments` (quando resolvido com sucesso).
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub instrument_id: Option<String>,

    /// Data do pregão em que o negócio ocorreu (YYYY-MM-DD).
    #[schema(example = "2026-03-01")]
    pub trade_date: NaiveDate,

    /// Código BDI (Boletim Diário de Informações) identificando a classe do papel (ex.: `02` = Lote Padrão, `12` = Fundos Imobiliários).
    #[schema(example = "02")]
    pub bdi_code: Option<String>,

    /// Símbolo de negociação (ticker) no pregão (ex.: `PETR4`).
    #[schema(example = "PETR4")]
    pub ticker_symbol: Option<String>,

    /// Tipo de mercado de negociação (ex.: `010` = À Vista, `020` = Fracionário, `070` = Opções de Compra).
    #[schema(example = "010")]
    pub market_type: Option<String>,

    /// Nome abreviado oficial da empresa emissora na B3.
    #[schema(example = "PETROBRAS")]
    pub issuer_short_name: Option<String>,

    /// Especificação do tipo de ação ou papel (ex.: `PN`, `ON`, `UNT`, `CI`).
    #[schema(example = "PN")]
    pub paper_specification: Option<String>,

    /// Prazo em dias para o mercado a termo.
    #[schema(example = "030")]
    pub term_days: Option<String>,

    /// Moeda de cotação oficial (ex: `R$`).
    #[schema(example = "R$")]
    pub currency: Option<String>,

    /// Preço bruto de abertura na sessão (unadjusted).
    #[schema(example = "35.50")]
    pub open_price: Option<Decimal>,

    /// Preço bruto máximo registrado na sessão (unadjusted).
    #[schema(example = "36.40")]
    pub high_price: Option<Decimal>,

    /// Preço bruto mínimo registrado na sessão (unadjusted).
    #[schema(example = "35.20")]
    pub low_price: Option<Decimal>,

    /// Preço médio ponderado pelo volume negociado na sessão (unadjusted).
    #[schema(example = "35.85")]
    pub average_price: Option<Decimal>,

    /// Preço bruto de fechamento praticado no último negócio (unadjusted).
    #[schema(example = "36.00")]
    pub last_price: Option<Decimal>,

    /// Melhor oferta de compra registrada no fechamento do pregão.
    #[schema(example = "35.98")]
    pub best_bid_price: Option<Decimal>,

    /// Melhor oferta de venda registrada no fechamento do pregão.
    #[schema(example = "36.00")]
    pub best_ask_price: Option<Decimal>,

    /// Quantidade total de negócios efetuados com o papel no pregão.
    #[schema(example = 45200)]
    pub trades_count: Option<i32>,

    /// Quantidade física total de ações ou cotas negociadas no pregão.
    #[schema(example = "15200000")]
    pub traded_quantity: Option<Decimal>,

    /// Volume financeiro total bruto negociado em Reais (BRL).
    #[schema(example = "547200000.00")]
    pub traded_volume: Option<Decimal>,

    /// Preço de exercício para contratos de opções.
    #[schema(example = "36.00")]
    pub option_exercise_price: Option<Decimal>,

    /// Indicador de correção de preços do contrato de opções.
    #[schema(example = "0")]
    pub price_correction_indicator: Option<String>,

    /// Data de vencimento do contrato (para opções e termos).
    #[schema(example = "2026-03-20")]
    pub maturity_date: Option<NaiveDate>,

    /// Fator de cotação do papel (ex: 1 = por ação individual, 1000 = por lote de mil ações).
    #[schema(example = 1)]
    pub quotation_factor: Option<i32>,

    /// Preço de exercício expresso em pontos (para opções referenciadas em índices).
    #[schema(example = "0.00")]
    pub exercise_price_points: Option<Decimal>,

    /// Código ISIN internacional de 12 dígitos.
    #[schema(example = "BRPETRACNPR6")]
    pub isin: Option<String>,

    /// Número de distribuição do papel (`DISMES`), discriminador para emissões concomitantes.
    #[schema(example = "100")]
    pub distribution_number: Option<String>,

    /// Status da amarração cadastral com a tabela de instrumentos (`resolved`, `unresolved`, `ambiguous`).
    #[schema(example = "resolved")]
    pub resolution_status: String,

    /// Metadados de proveniência do arquivo oficial COTAHIST.
    pub provenance: Provenance,

    /// Metadados operacionais internos exibidos quando `include_internal=true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_metadata: Option<InternalMetadata>,
}

/// Parâmetros de filtro para cotações históricas do COTAHIST.
#[derive(Debug, Clone, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CotahistFilterParams {
    /// Data da sessão de negociação a consultar (YYYY-MM-DD).
    #[param(example = "2026-03-01")]
    pub trade_date: Option<NaiveDate>,

    /// Símbolo de negociação (ticker, case-insensitive, ex: `PETR4`).
    #[param(example = "PETR4")]
    pub ticker: Option<String>,

    /// Código ISIN de 12 caracteres (ex: `BRPETRACNPR6`).
    #[param(example = "BRPETRACNPR6")]
    pub isin: Option<String>,

    /// Código BDI do papel (ex: `02` para Lote Padrão).
    #[param(example = "02")]
    pub bdi_code: Option<String>,

    /// Tipo de mercado (ex: `010` para À Vista).
    #[param(example = "010")]
    pub market_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes",
    tag = "B3 COTAHIST Historical Quotes",
    summary = "Listagem de Cotações Históricas (COTAHIST)",
    description = "Retorna cotações históricas diárias oficiais extraídas das séries históricas COTAHIST da B3 (arquivos anuais, mensais ou diários). \n\n### AVISO REGULATÓRIO IMPORTANTE SOBRE PREÇOS:\nOs preços fornecidos pelo arquivo COTAHIST representam **preços nominais históricos brutos (unadjusted)** praticados na data da negociação. Eles **NÃO sofrem ajuste retroativo** por eventos corporativos (dividendos, JCP, desdobramentos, grupamentos ou bonificações). \n\n### Chave Natural e Idempotência:\nA unicidade de registro inclui obrigatoriamente `(trade_date, ticker_symbol, isin, market_type, distribution_number)` protegida por `UNIQUE NULLS NOT DISTINCT`.",
    params(PaginationParams, CotahistFilterParams),
    responses(
        (status = 200, description = "Cotações históricas retornadas com sucesso.", body = PaginatedResponse<CotahistQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 500, description = "Erro interno ao processar a consulta no banco de dados.")
    )
)]
pub async fn list_cotahist_quotes(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationParams>,
    Query(filters): Query<CotahistFilterParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let limit = pagination.limit();
    let offset = pagination.offset();

    let ticker = filters.ticker.as_deref().map(|s| s.trim().to_uppercase());
    let isin = filters.isin.as_deref().map(|s| s.trim().to_uppercase());
    let bdi = filters.bdi_code.as_deref().map(|s| s.trim());
    let mkt = filters.market_type.as_deref().map(|s| s.trim());

    let count_query = r#"
        SELECT count(*)
        FROM b3_cotahist_quotes q
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.isin = $3)
          AND ($4::varchar IS NULL OR q.bdi_code = $4)
          AND ($5::varchar IS NULL OR q.market_type = $5)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(&isin)
        .bind(bdi)
        .bind(mkt)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.bdi_code, q.ticker_symbol,
            q.market_type, q.issuer_short_name, q.paper_specification, q.term_days,
            q.currency, q.open_price, q.high_price, q.low_price, q.average_price,
            q.last_price, q.best_bid_price, q.best_ask_price, q.trades_count,
            q.traded_quantity, q.traded_volume, q.option_exercise_price,
            q.price_correction_indicator, q.maturity_date, q.quotation_factor,
            q.exercise_price_points, q.isin, q.distribution_number, q.resolution_status,
            q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_cotahist_quotes q
        JOIN b3_source_files f ON f.id = q.source_file_id
        WHERE ($1::date IS NULL OR q.trade_date = $1)
          AND ($2::varchar IS NULL OR q.ticker_symbol = $2)
          AND ($3::varchar IS NULL OR q.isin = $3)
          AND ($4::varchar IS NULL OR q.bdi_code = $4)
          AND ($5::varchar IS NULL OR q.market_type = $5)
        ORDER BY q.trade_date DESC, q.ticker_symbol ASC
        LIMIT $6 OFFSET $7
    "#;

    let rows = sqlx::query(select_query)
        .bind(filters.trade_date)
        .bind(&ticker)
        .bind(&isin)
        .bind(bdi)
        .bind(mkt)
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

            CotahistQuoteItem {
                id: row.get("id"),
                instrument_id: row.get("instrument_id"),
                trade_date: row.get("trade_date"),
                bdi_code: row.get("bdi_code"),
                ticker_symbol: row.get("ticker_symbol"),
                market_type: row.get("market_type"),
                issuer_short_name: row.get("issuer_short_name"),
                paper_specification: row.get("paper_specification"),
                term_days: row.get("term_days"),
                currency: row.get("currency"),
                open_price: row.get("open_price"),
                high_price: row.get("high_price"),
                low_price: row.get("low_price"),
                average_price: row.get("average_price"),
                last_price: row.get("last_price"),
                best_bid_price: row.get("best_bid_price"),
                best_ask_price: row.get("best_ask_price"),
                trades_count: row.get("trades_count"),
                traded_quantity: row.get("traded_quantity"),
                traded_volume: row.get("traded_volume"),
                option_exercise_price: row.get("option_exercise_price"),
                price_correction_indicator: row.get("price_correction_indicator"),
                maturity_date: row.get("maturity_date"),
                quotation_factor: row.get("quotation_factor"),
                exercise_price_points: row.get("exercise_price_points"),
                isin: row.get("isin"),
                distribution_number: row.get("distribution_number"),
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
        disclaimer: Some(COTAHIST_DISCLAIMER.to_string()),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}",
    tag = "B3 COTAHIST Historical Quotes",
    summary = "Série Histórica COTAHIST por Ticker",
    description = "Retorna o histórico diário de cotações nominais não-ajustadas para um símbolo de negociação (ticker). Inclui o disclaimer obrigatório.",
    params(
        ("ticker_symbol" = String, Path, description = "Símbolo de negociação da B3 (ex: PETR4)", example = "PETR4"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Série histórica COTAHIST retornada com sucesso.", body = PaginatedResponse<CotahistQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_cotahist_by_ticker(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let filters = CotahistFilterParams {
        trade_date: None,
        ticker: Some(ticker),
        isin: None,
        bdi_code: None,
        market_type: None,
    };
    list_cotahist_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes/by-isin/{isin}",
    tag = "B3 COTAHIST Historical Quotes",
    summary = "Série Histórica COTAHIST por Código ISIN",
    description = "Retorna o histórico diário de cotações nominais não-ajustadas para o código ISIN internacional.",
    params(
        ("isin" = String, Path, description = "Código ISIN de 12 dígitos (ex: BRPETRACNPR6)", example = "BRPETRACNPR6"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Série histórica COTAHIST retornada com sucesso.", body = PaginatedResponse<CotahistQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_cotahist_by_isin(
    State(state): State<Arc<AppState>>,
    Path(isin): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let filters = CotahistFilterParams {
        trade_date: None,
        ticker: None,
        isin: Some(isin),
        bdi_code: None,
        market_type: None,
    };
    list_cotahist_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes/by-date/{trade_date}",
    tag = "B3 COTAHIST Historical Quotes",
    summary = "Cotações COTAHIST por Data de Pregão",
    description = "Retorna todos os papéis e cotações brutas negociadas em uma data específica do COTAHIST.",
    params(
        ("trade_date" = NaiveDate, Path, description = "Data da sessão de negociação (YYYY-MM-DD)", example = "2026-03-01"),
        PaginationParams
    ),
    responses(
        (status = 200, description = "Cotações do dia retornadas com sucesso.", body = PaginatedResponse<CotahistQuoteItem>),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido.")
    )
)]
pub async fn get_cotahist_by_date(
    State(state): State<Arc<AppState>>,
    Path(trade_date): Path<NaiveDate>,
    Query(pagination): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<CotahistQuoteItem>>, (StatusCode, String)> {
    let filters = CotahistFilterParams {
        trade_date: Some(trade_date),
        ticker: None,
        isin: None,
        bdi_code: None,
        market_type: None,
    };
    list_cotahist_quotes(State(state), Query(pagination), Query(filters)).await
}

#[utoipa::path(
    get,
    path = "/api/v1/b3/cotahist/quotes/{quote_id}",
    tag = "B3 COTAHIST Historical Quotes",
    summary = "Obter Cotação COTAHIST por UUID",
    description = "Recupera o registro detalhado de uma cotação COTAHIST através do seu identificador único (UUID).",
    params(
        ("quote_id" = String, Path, description = "UUID da cotação", example = "90a1b2c3-d4e5-6789-0123-456789abcdef")
    ),
    responses(
        (status = 200, description = "Cotação localizada com sucesso.", body = CotahistQuoteItem),
        (status = 401, description = "Não autorizado: Token Bearer ausente ou inválido."),
        (status = 404, description = "Cotação não encontrada para o ID informado.")
    )
)]
pub async fn get_cotahist_by_id(
    State(state): State<Arc<AppState>>,
    Path(quote_id): Path<String>,
) -> Result<Json<CotahistQuoteItem>, (StatusCode, String)> {
    let select_query = r#"
        SELECT 
            q.id, q.instrument_id, q.trade_date, q.bdi_code, q.ticker_symbol,
            q.market_type, q.issuer_short_name, q.paper_specification, q.term_days,
            q.currency, q.open_price, q.high_price, q.low_price, q.average_price,
            q.last_price, q.best_bid_price, q.best_ask_price, q.trades_count,
            q.traded_quantity, q.traded_volume, q.option_exercise_price,
            q.price_correction_indicator, q.maturity_date, q.quotation_factor,
            q.exercise_price_points, q.isin, q.distribution_number, q.resolution_status,
            q.source_file_id, q.source_member_name, q.source_line_number,
            q.data_changed_at, q.field_changed_at, q.last_seen_at,
            f.source_file_type, f.original_file_name, f.sha256, f.ingested_at
        FROM b3_cotahist_quotes q
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
            "Cotahist quote not found".to_string(),
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

    Ok(Json(CotahistQuoteItem {
        id: row.get("id"),
        instrument_id: row.get("instrument_id"),
        trade_date: row.get("trade_date"),
        bdi_code: row.get("bdi_code"),
        ticker_symbol: row.get("ticker_symbol"),
        market_type: row.get("market_type"),
        issuer_short_name: row.get("issuer_short_name"),
        paper_specification: row.get("paper_specification"),
        term_days: row.get("term_days"),
        currency: row.get("currency"),
        open_price: row.get("open_price"),
        high_price: row.get("high_price"),
        low_price: row.get("low_price"),
        average_price: row.get("average_price"),
        last_price: row.get("last_price"),
        best_bid_price: row.get("best_bid_price"),
        best_ask_price: row.get("best_ask_price"),
        trades_count: row.get("trades_count"),
        traded_quantity: row.get("traded_quantity"),
        traded_volume: row.get("traded_volume"),
        option_exercise_price: row.get("option_exercise_price"),
        price_correction_indicator: row.get("price_correction_indicator"),
        maturity_date: row.get("maturity_date"),
        quotation_factor: row.get("quotation_factor"),
        exercise_price_points: row.get("exercise_price_points"),
        isin: row.get("isin"),
        distribution_number: row.get("distribution_number"),
        resolution_status: row.get("resolution_status"),
        provenance,
        internal_metadata: None,
    }))
}
