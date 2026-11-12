//! COTAHIST analytics endpoints (derived indicators from nominal prices).
//!
//! All handlers document that results are derived and prices are unadjusted.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use utoipa::IntoParams;

use crate::api::health::AppState;
use crate::error::AppResult;
use crate::models::cotahist_analytics::*;
use crate::models::envelope::ApiResponse;
use crate::models::pagination::PaginationParams;
use crate::services::cotahist_analytics::CotahistAnalyticsService;

/// Período obrigatório `desde` + opcional `ate`.
#[derive(Debug, Deserialize, IntoParams)]
pub struct AnalyticsPeriodParams {
    /// Data inicial obrigatória (AAAA-MM-DD). Define o início da janela analítica sobre preços nominais COTAHIST.
    #[param(example = "2026-01-01")]
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD). Se omitida, usa todos os pregões disponíveis a partir de `desde`.
    #[param(example = "2026-01-31")]
    pub ate: Option<String>,
}

fn require_desde(params: &AnalyticsPeriodParams) -> AppResult<&str> {
    match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => Ok(d),
        _ => Err(crate::error::AppError::InvalidParam {
            message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
        }),
    }
}

/// Parâmetros de retorno relativo vs. benchmark.
#[derive(Debug, Deserialize, IntoParams)]
pub struct RelativeParams {
    /// Data inicial obrigatória (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD)
    pub ate: Option<String>,
    /// Benchmark. Atualmente apenas `IBOV` (série histórica ingerida).
    #[serde(default = "default_benchmark")]
    #[param(example = "IBOV")]
    pub benchmark: String,
}

fn default_benchmark() -> String {
    "IBOV".to_string()
}

/// Parâmetros de movers do pregão.
#[derive(Debug, Deserialize, IntoParams)]
pub struct MoversParams {
    /// Ranking: `gainers` (maior `var_pct`), `losers` (menor `var_pct`), `volume` (maior `vol_tot`) ou `trades` (maior `tot_neg`).
    #[serde(default = "default_ranking")]
    #[param(example = "gainers")]
    pub ranking: String,
    /// Quantidade máxima de itens (1–100, padrão 20)
    #[serde(default = "default_movers_limit")]
    #[param(example = 20)]
    pub limite: u32,
    /// Filtrar por tipo de mercado B3 (TPMERC), quando disponível no Parquet
    #[param(example = 10)]
    pub market_type: Option<u8>,
}

fn default_ranking() -> String {
    "gainers".to_string()
}
fn default_movers_limit() -> u32 {
    20
}

/// Parâmetros da tela de liquidez.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LiquidityParams {
    /// Data inicial obrigatória (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD)
    pub ate: Option<String>,
    /// Filtrar por TPMERC
    pub market_type: Option<u8>,
    /// Exige ADV financeira mínima (média de `vol_tot` em R$)
    pub vol_tot_medio_min: Option<f64>,
    /// Número da página (1-indexado)
    #[serde(default = "default_page")]
    pub pagina: u32,
    /// Itens por página (máximo 200)
    #[serde(default = "default_per_page")]
    pub por_pagina: u32,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    50
}

/// Parâmetros de gaps.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GapsParams {
    /// Data inicial obrigatória (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD)
    pub ate: Option<String>,
    /// Filtra gaps com `|gap_pct| >=` este valor (percentual). Ex: `1.0` = gaps de pelo menos 1%.
    pub gap_pct_min: Option<f64>,
}

/// Parâmetros de indicadores móveis.
#[derive(Debug, Deserialize, IntoParams)]
pub struct RollingParams {
    /// Data inicial obrigatória (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD)
    pub ate: Option<String>,
    /// Janela em pregões (2–252, padrão 20) para SMA, volatilidade e ADV. EMA usa α = 2/(janela+1).
    #[serde(default = "default_janela")]
    #[param(example = 20)]
    pub janela: u32,
}

fn default_janela() -> u32 {
    20
}

/// Parâmetros de comparação multi-ticker.
#[derive(Debug, Deserialize, IntoParams)]
pub struct CompareParams {
    /// Lista de tickers separados por vírgula (máximo 10). Ex: `VALE3,PETR4,ITUB4`
    #[param(example = "VALE3,PETR4")]
    pub tickers: Option<String>,
    /// Data inicial obrigatória (AAAA-MM-DD)
    pub desde: Option<String>,
    /// Data final opcional (AAAA-MM-DD)
    pub ate: Option<String>,
}

/// Retorno acumulado do ticker no período (preços nominais)
///
/// Calcula o retorno simples e, quando há pelo menos dois pregões, o retorno
/// anualizado (252 pregões/ano) a partir de `pre_ult` COTAHIST.
///
/// **Origem:** arquivo histórico COTAHIST (B3), preços de fechamento nominais.
/// **Não inclui** ajuste retroativo por eventos corporativos (splits, proventos etc.).
/// **Indicadores derivados:** este endpoint não republica série oficial da B3.
///
/// Ordenação implícita: primeiro e último pregão com `pre_ult` válido no intervalo.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/returns",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker (ex: VALE3)", example = "VALE3"),
        AnalyticsPeriodParams,
    ),
    responses(
        (status = 200, description = "Retorno do período calculado com sucesso", body = ReturnsApiResponse),
        (status = 400, description = "Parâmetro ausente, data inválida ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou dados insuficientes no período", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_returns_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<AnalyticsPeriodParams>,
) -> AppResult<Json<ApiResponse<ReturnsResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_returns(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Cartão estatístico do ticker no período
///
/// Agrega extremos de fechamento, retorno, volatilidade realizada anualizada,
/// ATR aproximado, médias/medianas de liquidez e fração de pregões sem negócio.
///
/// **Aviso COTAHIST:** preços nominais sem ajuste por eventos corporativos; métricas são derivadas.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/stats",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        AnalyticsPeriodParams,
    ),
    responses(
        (status = 200, description = "Estatísticas calculadas", body = StatsApiResponse),
        (status = 400, description = "Parâmetro ausente, data inválida ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados no período", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_stats_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<AnalyticsPeriodParams>,
) -> AppResult<Json<ApiResponse<StatsResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_stats(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Drawdown máximo sobre fechamentos nominais
///
/// Percorre a série de `pre_ult` e identifica o maior drawdown pico→vale, além do
/// drawdown atual em relação ao máximo do período.
///
/// **Aviso COTAHIST:** série nominal; eventos corporativos podem distorcer drawdowns históricos.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/drawdown",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        AnalyticsPeriodParams,
    ),
    responses(
        (status = 200, description = "Drawdown calculado", body = DrawdownApiResponse),
        (status = 400, description = "Parâmetro ausente, data inválida ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou dados insuficientes", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_drawdown_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<AnalyticsPeriodParams>,
) -> AppResult<Json<ApiResponse<DrawdownResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_drawdown(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Posição do fechamento atual no intervalo mínimo–máximo do período
///
/// Útil como proxy de “posição na banda de 52 semanas” quando `desde`/`ate`
/// cobrem aproximadamente um ano de pregões. `posicao_pct` = 100 no máximo e 0 no mínimo.
///
/// **Aviso COTAHIST:** preços nominais sem ajuste retroativo.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/range-position",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        AnalyticsPeriodParams,
    ),
    responses(
        (status = 200, description = "Posição no intervalo retornada", body = RangePositionApiResponse),
        (status = 400, description = "Parâmetro ausente, data inválida ou intervalo invertido", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_range_position_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<AnalyticsPeriodParams>,
) -> AppResult<Json<ApiResponse<RangePositionResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_range_position(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Performance relativa versus IBOV (datas alinhadas)
///
/// Cruza pregões do ticker (COTAHIST) com a série IBOV ingerida, usando apenas
/// datas comuns com fechamento válido. Reporta retorno do ativo, do benchmark e o excesso.
///
/// **Benchmark:** somente `IBOV` nesta versão.
/// **Aviso:** preços nominais; excesso de retorno é indicador derivado.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/relative",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        RelativeParams,
    ),
    responses(
        (status = 200, description = "Performance relativa calculada", body = RelativeApiResponse),
        (status = 400, description = "Parâmetro inválido ou benchmark não suportado", body = ErrorResponse),
        (status = 404, description = "Ticker/IBOV sem interseção suficiente de datas", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_relative_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<RelativeParams>,
) -> AppResult<Json<ApiResponse<RelativeResponse>>> {
    let start = Instant::now();
    let desde = match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => d,
        _ => {
            return Err(crate::error::AppError::InvalidParam {
                message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
            })
        }
    };
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_relative(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
        &params.benchmark,
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Maiores altas/baixas e líderes de volume/negócios do pregão
///
/// Classifica tickers em uma data de pregão por variação vs. pregão anterior
/// (`gainers`/`losers`) ou por liquidez (`volume`/`trades`).
///
/// `var_pct`/`var_abs` são derivados de PREULT nominais (não oficiais).
#[utoipa::path(
    get,
    path = "/cotahist/dates/{date}/movers",
    tag = "cotahist",
    params(
        ("date" = String, Path, description = "Data do pregão (AAAA-MM-DD)", example = "2026-01-02"),
        MoversParams,
    ),
    responses(
        (status = 200, description = "Movers retornados", body = MoversApiResponse),
        (status = 400, description = "Data ou ranking inválidos", body = ErrorResponse),
        (status = 404, description = "Nenhum dado para a data", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_movers_handler(
    State(state): State<Arc<AppState>>,
    Path(date): Path<String>,
    Query(params): Query<MoversParams>,
) -> AppResult<Json<ApiResponse<MoversResponse>>> {
    let start = Instant::now();
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_movers(
        &query_service,
        &date,
        &params.ranking,
        params.limite,
        params.market_type,
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Tela de liquidez (ADV / negócios) entre tickers
///
/// Agrupa COTAHIST por ticker no período e ordena por volume financeiro médio
/// decrescente. Suporta filtro de `market_type` e piso de ADV.
///
/// Indicadores são operacionais/derivados a partir de VOLTOT, QUATTOT e TOTNEG.
#[utoipa::path(
    get,
    path = "/cotahist/liquidity",
    tag = "cotahist",
    params(LiquidityParams),
    responses(
        (status = 200, description = "Tela de liquidez retornada", body = LiquidityScreenApiResponse),
        (status = 400, description = "Parâmetro ausente ou data inválida", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_liquidity_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LiquidityParams>,
) -> AppResult<Json<ApiResponse<LiquidityScreenResponse>>> {
    let start = Instant::now();
    let desde = match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => d,
        _ => {
            return Err(crate::error::AppError::InvalidParam {
                message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
            })
        }
    };
    let pagination = PaginationParams {
        pagina: params.pagina,
        por_pagina: params.por_pagina,
    }
    .validated();
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_liquidity_screen(
        &query_service,
        desde,
        params.ate.as_deref(),
        params.market_type,
        params.vol_tot_medio_min,
        &pagination,
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Gaps de abertura versus fechamento anterior
///
/// Para cada pregão com `pre_abr` e `pre_ult` anterior válidos, calcula gap absoluto
/// e percentual. Opcionalmente filtra por magnitude mínima e indica se o gap foi
/// “preenchido” pela máxima/mínima do dia.
///
/// **Aviso COTAHIST:** preços nominais.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/gaps",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        GapsParams,
    ),
    responses(
        (status = 200, description = "Gaps retornados", body = GapsApiResponse),
        (status = 400, description = "Parâmetro ausente ou data inválida", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou dados insuficientes", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_gaps_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<GapsParams>,
) -> AppResult<Json<ApiResponse<GapsResponse>>> {
    let start = Instant::now();
    let desde = match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => d,
        _ => {
            return Err(crate::error::AppError::InvalidParam {
                message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
            })
        }
    };
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_gaps(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
        params.gap_pct_min,
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Indicadores móveis derivados (SMA, EMA, volatilidade, ADV)
///
/// Série diária com médias móveis de fechamento, volatilidade realizada anualizada
/// na janela e média móvel de volume financeiro. Valores móveis ficam nulos até a
/// janela estar completa (EMA é seedada no primeiro fechamento).
///
/// **Não são** séries oficiais B3 — analytics derivadas de COTAHIST nominal.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/rolling",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        RollingParams,
    ),
    responses(
        (status = 200, description = "Série de indicadores móveis", body = RollingApiResponse),
        (status = 400, description = "Parâmetro ausente ou data inválida", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_rolling_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<RollingParams>,
) -> AppResult<Json<ApiResponse<RollingResponse>>> {
    let start = Instant::now();
    let desde = match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => d,
        _ => {
            return Err(crate::error::AppError::InvalidParam {
                message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
            })
        }
    };
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_rolling(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
        params.janela,
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Spread compra/venda a partir das ofertas COTAHIST
///
/// Usa `pre_ofe_compra` e `pre_ofe_venda` do registro diário (quando presentes)
/// para calcular spread absoluto, mid e spread percentual.
///
/// **Limitação:** ofertas de fechamento COTAHIST; não é book em tempo real.
#[utoipa::path(
    get,
    path = "/cotahist/tickers/{ticker}/spread",
    tag = "cotahist",
    params(
        ("ticker" = String, Path, description = "Código do ticker", example = "VALE3"),
        AnalyticsPeriodParams,
    ),
    responses(
        (status = 200, description = "Série de spreads", body = SpreadApiResponse),
        (status = 400, description = "Parâmetro ausente ou data inválida", body = ErrorResponse),
        (status = 404, description = "Ticker não encontrado ou sem dados", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_spread_handler(
    State(state): State<Arc<AppState>>,
    Path(ticker): Path<String>,
    Query(params): Query<AnalyticsPeriodParams>,
) -> AppResult<Json<ApiResponse<SpreadResponse>>> {
    let start = Instant::now();
    let desde = require_desde(&params)?;
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::get_spread(
        &query_service,
        &state.data_loader.ticker_registry,
        &ticker,
        desde,
        params.ate.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Comparação normalizada (base 100) entre tickers
///
/// Alinha tickers pela interseção de datas com `pre_ult` válido e normaliza cada
/// série para 100 na primeira data comum. Máximo de 10 tickers por chamada.
///
/// **Aviso COTAHIST:** preços nominais; a normalização é derivada.
#[utoipa::path(
    get,
    path = "/cotahist/compare",
    tag = "cotahist",
    params(CompareParams),
    responses(
        (status = 200, description = "Comparação normalizada retornada", body = CompareApiResponse),
        (status = 400, description = "Lista de tickers ou datas inválidas", body = ErrorResponse),
        (status = 404, description = "Sem interseção suficiente de datas", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = [])),
)]
pub(crate) async fn get_compare_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CompareParams>,
) -> AppResult<Json<ApiResponse<CompareResponse>>> {
    let start = Instant::now();
    let desde = match params.desde.as_deref().map(str::trim) {
        Some(d) if !d.is_empty() => d,
        _ => {
            return Err(crate::error::AppError::InvalidParam {
                message: "O parâmetro 'desde' é obrigatório (AAAA-MM-DD).".to_string(),
            })
        }
    };
    let tickers = params.tickers.as_deref().unwrap_or("");
    let query_service = state.data_loader.get_query_service().await;
    let data = CotahistAnalyticsService::compare(
        &query_service,
        &state.data_loader.ticker_registry,
        tickers,
        desde,
        params.ate.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Analytics routes under `/cotahist/...`.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/cotahist/compare", get(get_compare_handler))
        .route("/cotahist/liquidity", get(get_liquidity_handler))
        .route(
            "/cotahist/tickers/:ticker/returns",
            get(get_returns_handler),
        )
        .route("/cotahist/tickers/:ticker/stats", get(get_stats_handler))
        .route(
            "/cotahist/tickers/:ticker/drawdown",
            get(get_drawdown_handler),
        )
        .route(
            "/cotahist/tickers/:ticker/range-position",
            get(get_range_position_handler),
        )
        .route(
            "/cotahist/tickers/:ticker/relative",
            get(get_relative_handler),
        )
        .route("/cotahist/tickers/:ticker/gaps", get(get_gaps_handler))
        .route(
            "/cotahist/tickers/:ticker/rolling",
            get(get_rolling_handler),
        )
        .route("/cotahist/tickers/:ticker/spread", get(get_spread_handler))
        .route("/cotahist/dates/:date/movers", get(get_movers_handler))
        .with_state(state)
}
