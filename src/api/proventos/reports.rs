//! HTTP adapters for data-com reports.

use super::{map_proventos_error, ProventosQueryParams};
use crate::api::health::AppState;
#[allow(unused_imports)]
use crate::error::ErrorResponse;
use crate::error::{AppError, AppResult};
use crate::models::envelope::ApiResponse;
#[allow(unused_imports)]
use crate::models::envelope::{
    ProventosPeriodApiResponse, ProventosSeriesApiResponse, ProventosYearlyApiResponse,
};
use crate::models::proventos::{
    ProventosPeriodQuery, ProventosPeriodTotal, ProventosTickerSeries, ProventosYearlyQuery,
    ProventosYearlyResponse,
};
use crate::services::proventos::ProventosError;
use axum::{extract::State, Json};
use serde::Serialize;
use std::{sync::Arc, time::Instant};

/// Consultar o total anual de proventos
///
/// Soma os valores unitários de Dividendo, JCP e Rendimento do ticker pela
/// data-com entre 1 de janeiro e 31 de dezembro do ano informado, inclusive.
/// Lançamentos sem data-com ficam fora do cálculo. A resposta traz um único
/// objeto com `proventos_on_year` em reais, como texto com seis casas decimais.
/// Quando não há registros para o ticker e ano, o total é `"0.000000"`.
/// O cálculo usa os registros cadastrados e não mede rendimento percentual.
#[utoipa::path(
    get,
    path = "/proventos/yearly",
    tag = "proventos",
    params(ProventosYearlyQuery),
    responses(
        (status = 200, description = "Total anual de um ticker em data.proventos_on_year", body = ProventosYearlyApiResponse),
        (status = 400, description = "Ticker ou ano ausente ou inválido, ou parâmetro desconhecido", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida quando a autenticação está habilitada", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro ao consultar os dados no ledger", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn yearly(
    State(state): State<Arc<AppState>>,
    ProventosQueryParams(query): ProventosQueryParams<ProventosYearlyQuery>,
) -> AppResult<Json<ApiResponse<ProventosYearlyResponse>>> {
    run_report(move || state.proventos.yearly(query)).await
}

/// Consultar o total de proventos por ticker em um período
///
/// Soma Dividendo, JCP e Rendimento em reais por ação ou cota para cada ticker
/// solicitado. O intervalo `desde` a `ate` é inclusivo e filtra pela data-com.
/// Registros sem data-com ficam fora do cálculo. A resposta contém um objeto por
/// ticker distinto, na ordem da primeira ocorrência após normalização.
/// `proventos_on_period` é um decimal textual de seis casas e vale `"0.000000"`
/// quando não há registros. A resposta não é paginada.
#[utoipa::path(
    get,
    path = "/proventos/period",
    tag = "proventos",
    params(ProventosPeriodQuery),
    responses(
        (status = 200, description = "Lista de totais por ticker em data", body = ProventosPeriodApiResponse),
        (status = 400, description = "Seleção de tickers ou datas ausente ou inválida, intervalo invertido ou parâmetro desconhecido", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida quando a autenticação está habilitada", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro ao consultar os dados no ledger", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn period(
    State(state): State<Arc<AppState>>,
    ProventosQueryParams(query): ProventosQueryParams<ProventosPeriodQuery>,
) -> AppResult<Json<ApiResponse<Vec<ProventosPeriodTotal>>>> {
    run_report(move || state.proventos.period(query)).await
}

/// Consultar séries compactas de proventos para gráficos
///
/// Retorna uma série por ticker distinto, na ordem da primeira ocorrência. Cada
/// ponto reúne a soma de Dividendo, JCP e Rendimento de uma data-com no intervalo
/// inclusivo. `date_ms` representa a meia-noite UTC e `valor` traz o total unitário
/// em reais como texto com seis casas decimais. Os pontos são ordenados por data
/// crescente. Dias sem lançamentos são omitidos e tickers sem registros têm
/// `series: []`. A resposta não é paginada.
#[utoipa::path(
    get,
    path = "/proventos/series",
    tag = "proventos",
    params(ProventosPeriodQuery),
    responses(
        (status = 200, description = "Lista de séries compactas por ticker em data", body = ProventosSeriesApiResponse),
        (status = 400, description = "Seleção de tickers ou datas ausente ou inválida, intervalo invertido ou parâmetro desconhecido", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida quando a autenticação está habilitada", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro ao consultar os dados no ledger", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn series(
    State(state): State<Arc<AppState>>,
    ProventosQueryParams(query): ProventosQueryParams<ProventosPeriodQuery>,
) -> AppResult<Json<ApiResponse<Vec<ProventosTickerSeries>>>> {
    run_report(move || state.proventos.series(query)).await
}

async fn run_report<T: Serialize + Send + 'static>(
    report: impl FnOnce() -> Result<T, ProventosError> + Send + 'static,
) -> AppResult<Json<ApiResponse<T>>> {
    let start = Instant::now();
    let data = tokio::task::spawn_blocking(report)
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .map_err(map_proventos_error)?;
    Ok(Json(ApiResponse::new(
        data,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}
