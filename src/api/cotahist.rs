//! COTAHIST API endpoints.
//!
//! All endpoints are under /api/v1/cotahist/.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use utoipa::IntoParams;

use crate::api::health::AppState;
use crate::error::AppResult;
use crate::models::cotahist::*;
use crate::models::envelope::ApiResponse;
use crate::models::pagination::{PaginationMeta, PaginationParams};
use crate::services::cotahist::CotahistService;

/// Parâmetros de consulta para listar tickers.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListTickersParams {
    /// Número da página (1-indexado)
    #[serde(default = "default_page")]
    pub pagina: u32,
    /// Itens por página (máximo 200)
    #[serde(default = "default_per_page")]
    pub por_pagina: u32,
    /// Filtrar por prefixo do ticker (ex: "VALE")
    pub busca: Option<String>,
    /// Apenas tickers ativos (padrão: true)
    #[serde(default = "default_true")]
    pub ativo: bool,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    50
}
fn default_true() -> bool {
    true
}

/// Listar todos os tickers disponíveis
///
/// Retorna uma lista paginada de todos os tickers disponíveis nos dados ingeridos. Suporta busca por prefixo e filtro por status ativo.
#[utoipa::path(
    get,
    path = "/cotahist/tickers",
    tag = "cotahist",
    params(
        ListTickersParams,
    ),
    responses(
        (status = 200, description = "Lista de tickers retornada com sucesso", body = TickerListApiResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn list_tickers_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListTickersParams>,
) -> AppResult<Json<ApiResponse<TickerListResponse>>> {
    let start = Instant::now();

    let pagina = params.pagina.max(1);
    let por_pagina = params.por_pagina.clamp(1, 200);

    let (tickers, total) = CotahistService::list_tickers(
        &state.data_loader.ticker_registry,
        pagina,
        por_pagina,
        params.busca.as_deref(),
        params.ativo,
    )?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(
        TickerListResponse {
            tickers,
            paginacao: PaginationMeta::new(total, pagina, por_pagina),
        },
        elapsed,
    )))
}

/// Obter metadados de um ticker
///
/// Retorna informações detalhadas de um ticker específico, incluindo ISIN, nome da empresa, datas de primeira e última cotação, e preço mais recente.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker (ex: VALE3, PETR4)"),
    ),
    responses(
        (status = 200, description = "Ticker encontrado", body = TickerDetailApiResponse),
        (status = 404, description = "Ticker não encontrado", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_ticker_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> AppResult<Json<ApiResponse<TickerDetailResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let detail = CotahistService::get_ticker_detail(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(detail, elapsed)))
}

/// Parâmetros de consulta para histórico de cotações.
#[derive(Debug, Deserialize, IntoParams)]
pub struct QuoteHistoryParams {
    /// Data inicial (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final (AAAA-MM-DD)
    pub ate: Option<String>,
    /// Últimos N dias de pregão
    pub dias: Option<u32>,
    /// Ordem de classificação: asc ou desc (padrão: desc)
    #[serde(default = "default_order")]
    pub ordem: String,
    /// Número da página
    #[serde(default = "default_page")]
    pub pagina: u32,
    /// Itens por página (padrão: 252, máximo: 1000)
    #[serde(default = "default_per_page_quotes")]
    pub por_pagina: u32,
}

fn default_order() -> String {
    "desc".to_string()
}
fn default_per_page_quotes() -> u32 {
    252
}

/// Obter histórico de cotações de um ticker
///
/// Retorna o histórico de cotações diárias de um ticker com filtros de data e paginação. Inclui variação percentual e absoluta calculadas automaticamente.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/quotes",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker"),
        QuoteHistoryParams,
    ),
    responses(
        (status = 200, description = "Cotações retornadas com sucesso", body = QuoteListApiResponse),
        (status = 400, description = "Data inválida ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_quotes_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<QuoteHistoryParams>,
) -> AppResult<Json<ApiResponse<QuoteListResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let pagination = PaginationParams {
        pagina: params.pagina,
        por_pagina: params.por_pagina,
    }
    .validated();

    let (quotes, total) = CotahistService::get_quotes(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        params.desde.as_deref(),
        params.ate.as_deref(),
        params.dias,
        &params.ordem,
        &pagination,
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(
        QuoteListResponse {
            ticker: ticker.to_uppercase(),
            quotes,
            paginacao: PaginationMeta::new(total, pagination.pagina, pagination.por_pagina),
        },
        elapsed,
    )))
}

/// Obter a cotação mais recente de um ticker
///
/// Retorna a cotação mais recente de um ticker, incluindo variação percentual e absoluta em relação ao pregão anterior.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/latest",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker"),
    ),
    responses(
        (status = 200, description = "Cotação mais recente retornada", body = LatestQuoteApiResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_latest_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
) -> AppResult<Json<ApiResponse<LatestQuoteResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let latest = CotahistService::get_latest_quote(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(latest, elapsed)))
}

/// Obter todas as cotações de uma data específica
///
/// Retorna todas as cotações de todos os tickers para uma data específica de pregão.
#[utoipa::path(
    get,
    path = "/cotahist/dates/{date}/quotes",
    tag = "cotahist",
    params(
        ("date" = String, Path, description = "Data do pregão (AAAA-MM-DD)"),
        PaginationParams,
    ),
    responses(
        (status = 200, description = "Cotações da data retornadas", body = DateQuotesApiResponse),
        (status = 400, description = "Formato de data inválido", body = ErrorResponse),
        (status = 404, description = "Nenhum dado encontrado para a data", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_date_quotes_handler(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<ApiResponse<DateQuotesResponse>>> {
    let start = Instant::now();

    let query_service = state.data_loader.get_query_service().await;

    let pagination = params.validated();

    let (quotes, total) =
        CotahistService::get_quotes_by_date(&query_service, &date, &pagination).await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(
        DateQuotesResponse {
            date,
            quotes,
            paginacao: PaginationMeta::new(total, pagination.pagina, pagination.por_pagina),
        },
        elapsed,
    )))
}

/// Parâmetros de consulta para série histórica compacta e extremos de PREULT.
#[derive(Debug, Deserialize, IntoParams)]
pub struct SeriesPeriodParams {
    /// Data inicial obrigatória (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD). Se omitida, usa todos os pregões a partir de `desde`.
    pub ate: Option<String>,
}

fn require_desde(params: &SeriesPeriodParams) -> AppResult<&str> {
    match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => Ok(d),
        _ => Err(crate::error::AppError::InvalidParam {
            message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
        }),
    }
}

/// Obter série histórica compacta para gráficos
///
/// Retorna uma lista enxuta com data em milissegundos (Unix epoch UTC 00:00), preço de
/// fechamento (`pre_ult`), volume total (`vol_tot`) e quantidade de negócios (`tot_neg`).
/// Projetada para renderização eficiente de gráficos históricos. Ordenação ascendente por data.
/// Sem paginação: o payload já é reduzido aos campos essenciais.
///
/// **Aviso COTAHIST:** preços nominais históricos sem ajuste retroativo por eventos corporativos.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/series",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker (ex: VALE3, PETR4)"),
        SeriesPeriodParams,
    ),
    responses(
        (status = 200, description = "Série histórica retornada com sucesso", body = SeriesApiResponse),
        (status = 400, description = "Data inválida, parâmetro ausente ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_series_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<SeriesPeriodParams>,
) -> AppResult<Json<ApiResponse<SeriesResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;

    let query_service = state.data_loader.get_query_service().await;

    let series = CotahistService::get_series(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(series, elapsed)))
}

/// Obter o menor preço de fechamento (PREULT) no período
///
/// Retorna o mínimo de `pre_ult` entre `desde` e `ate` (opcional), incluindo a data do pregão
/// em que o extremo ocorreu. Em caso de empate, retorna a data mais antiga.
///
/// **Aviso COTAHIST:** preços nominais históricos sem ajuste retroativo por eventos corporativos.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/pre-ult/min",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker (ex: VALE3, PETR4)"),
        SeriesPeriodParams,
    ),
    responses(
        (status = 200, description = "Mínimo de pre_ult retornado", body = PreUltExtremeApiResponse),
        (status = 400, description = "Data inválida, parâmetro ausente ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados no período", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_pre_ult_min_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<SeriesPeriodParams>,
) -> AppResult<Json<ApiResponse<PreUltExtremeResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;

    let query_service = state.data_loader.get_query_service().await;

    let extreme = CotahistService::get_pre_ult_min(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(extreme, elapsed)))
}

/// Obter o maior preço de fechamento (PREULT) no período
///
/// Retorna o máximo de `pre_ult` entre `desde` e `ate` (opcional), incluindo a data do pregão
/// em que o extremo ocorreu. Em caso de empate, retorna a data mais antiga.
///
/// **Aviso COTAHIST:** preços nominais históricos sem ajuste retroativo por eventos corporativos.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/pre-ult/max",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker (ex: VALE3, PETR4)"),
        SeriesPeriodParams,
    ),
    responses(
        (status = 200, description = "Máximo de pre_ult retornado", body = PreUltExtremeApiResponse),
        (status = 400, description = "Data inválida, parâmetro ausente ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados no período", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn get_pre_ult_max_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<SeriesPeriodParams>,
) -> AppResult<Json<ApiResponse<PreUltExtremeResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;

    let query_service = state.data_loader.get_query_service().await;

    let extreme = CotahistService::get_pre_ult_max(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    Ok(Json(ApiResponse::new(extreme, elapsed)))
}

/// Build COTAHIST routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/cotahist/tickers", get(list_tickers_handler))
        .route("/cotahist/tickers/:ticker", get(get_ticker_handler))
        .route("/cotahist/tickers/:ticker/quotes", get(get_quotes_handler))
        .route("/cotahist/tickers/:ticker/latest", get(get_latest_handler))
        .route("/cotahist/tickers/:ticker/series", get(get_series_handler))
        .route(
            "/cotahist/tickers/:ticker/pre-ult/min",
            get(get_pre_ult_min_handler),
        )
        .route(
            "/cotahist/tickers/:ticker/pre-ult/max",
            get(get_pre_ult_max_handler),
        )
        .route("/cotahist/dates/:date/quotes", get(get_date_quotes_handler))
        .with_state(state)
}
