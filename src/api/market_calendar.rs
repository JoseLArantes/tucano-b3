//! B3 market calendar and Trader negotiation-hours endpoints.

use crate::api::health::AppState;
#[allow(unused_imports)]
use crate::error::ErrorResponse;
use crate::error::{AppError, AppResult};
use crate::models::envelope::ApiResponse;
#[allow(unused_imports)]
use crate::models::envelope::{
    MarketCalendarApiResponse, MarketCalendarDeleteApiResponse, MarketCalendarYearsApiResponse,
    NegotiationHoursApiResponse,
};
use crate::models::market_calendar::{
    MarketCalendarCreateRequest, MarketCalendarDeleteResponse, MarketCalendarResponse,
    MarketCalendarUpdateRequest, MarketCalendarYearsResponse, NegotiationHoursResponse,
    NegotiationSessionHours,
};
use crate::services::market_calendar::MarketCalendarError;
use axum::extract::{FromRequest, Path, Request, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Instant;

const NEGOTIATION_HOURS_SOURCE: &str = "https://www.b3.com.br/pt_br/solucoes/plataformas/interface-de-negociacao/trader/horarios-de-negociacao.htm";

/// JSON body extractor that returns malformed calendar requests in the API error envelope.
pub struct CalendarJson<T>(T);

#[axum::async_trait]
impl<S, T> FromRequest<S> for CalendarJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|error| AppError::InvalidParam {
                message: format!("Corpo JSON inválido: {error}"),
            })
    }
}

fn map_calendar_error(error: MarketCalendarError) -> AppError {
    match error {
        MarketCalendarError::NotFound { year } => AppError::ResourceNotFound {
            resource: format!("calendário B3 de {year}"),
        },
        MarketCalendarError::AlreadyExists { year } => AppError::Conflict {
            message: format!("já existe um calendário para {year}"),
        },
        MarketCalendarError::Invalid(message) => AppError::InvalidParam { message },
        error @ (MarketCalendarError::Ledger(_) | MarketCalendarError::Serialization(_)) => {
            AppError::Internal(anyhow::Error::new(error))
        }
    }
}

fn require_configured_auth(state: &AppState) -> AppResult<()> {
    if state.auth_enabled {
        Ok(())
    } else {
        Err(AppError::ServiceUnavailable)
    }
}

/// Listar anos com calendário B3 cadastrado
///
/// Retorna os anos disponíveis em ordem crescente dentro de `data.anos`.
///
/// O calendário de 2026 é carregado uma vez por ledger a partir das fontes B3
/// publicadas. A resposta usa o envelope padrão `{ data, meta }`; a operação
/// exige Bearer ou API Key quando a autenticação está habilitada no serviço.
#[utoipa::path(
    get,
    path = "/calendar",
    tag = "calendário",
    responses(
        (status = 200, description = "Anos disponíveis", body = MarketCalendarYearsApiResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn list_calendars(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<ApiResponse<MarketCalendarYearsResponse>>> {
    let start = Instant::now();
    let anos = state.market_calendar.years().map_err(map_calendar_error)?;
    Ok(Json(ApiResponse::new(
        MarketCalendarYearsResponse { anos },
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Consultar um calendário anual da B3
///
/// Retorna feriados e exceções de funcionamento do ano informado, com as fontes
/// e o instante da última atualização. Datas e horários são locais da B3.
///
/// `situacao` diferencia `closed`, `special_hours` e `open`. Eventos vêm em
/// ordem crescente por data e o corpo é envolvido por `{ data, meta }`.
#[utoipa::path(
    get,
    path = "/calendar/{year}",
    tag = "calendário",
    params(("year" = i32, Path, description = "Ano civil do calendário (1900–2200)", example = 2026)),
    responses(
        (status = 200, description = "Calendário encontrado", body = MarketCalendarApiResponse),
        (status = 404, description = "Não há calendário cadastrado para o ano", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn get_calendar(
    State(state): State<Arc<AppState>>,
    Path(year): Path<i32>,
) -> AppResult<Json<ApiResponse<MarketCalendarResponse>>> {
    let start = Instant::now();
    let calendar = state
        .market_calendar
        .get(year)
        .map_err(map_calendar_error)?;
    Ok(Json(ApiResponse::new(
        calendar,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Criar calendário B3 para um ano
///
/// Cria um calendário que ainda não existe. Informe as fontes e os eventos em
/// datas válidas do ano; o registro é persistido no ledger SQLite.
///
/// O ano precisa estar entre 1900 e 2200, as datas precisam pertencer a ele e
/// não podem se repetir, e as fontes precisam apontar para `b3.com.br`. Evento com
/// horário excepcional usa `special_hours` e informa pelo menos um horário.
/// Retorna `409` se o ano já existir. Escritas exigem credencial configurada.
#[utoipa::path(
    post,
    path = "/calendar",
    tag = "calendário",
    request_body = MarketCalendarCreateRequest,
    responses(
        (status = 201, description = "Calendário criado", body = MarketCalendarApiResponse),
        (status = 400, description = "Calendário inválido", body = ErrorResponse),
        (status = 409, description = "Já existe um calendário para o ano", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de alterar calendários", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn create_calendar(
    State(state): State<Arc<AppState>>,
    CalendarJson(request): CalendarJson<MarketCalendarCreateRequest>,
) -> AppResult<(StatusCode, Json<ApiResponse<MarketCalendarResponse>>)> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    let calendar = state
        .market_calendar
        .create(request)
        .map_err(map_calendar_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(
            calendar,
            start.elapsed().as_secs_f64() * 1000.0,
        )),
    ))
}

/// Substituir calendário B3 de um ano
///
/// Substitui integralmente as fontes e os eventos de um calendário existente.
///
/// Todas as datas devem pertencer ao ano informado, as datas devem ser únicas
/// e ordenadas automaticamente pela API. O corpo anterior é substituído por
/// inteiro numa operação no ledger SQLite; o envelope retorna o novo calendário.
/// Retorna `404` quando o ano não existe e exige credencial configurada.
#[utoipa::path(
    put,
    path = "/calendar/{year}",
    tag = "calendário",
    params(("year" = i32, Path, description = "Ano civil do calendário (1900–2200)", example = 2026)),
    request_body = MarketCalendarUpdateRequest,
    responses(
        (status = 200, description = "Calendário atualizado", body = MarketCalendarApiResponse),
        (status = 400, description = "Calendário inválido", body = ErrorResponse),
        (status = 404, description = "Não há calendário cadastrado para o ano", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de alterar calendários", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn update_calendar(
    State(state): State<Arc<AppState>>,
    Path(year): Path<i32>,
    CalendarJson(request): CalendarJson<MarketCalendarUpdateRequest>,
) -> AppResult<Json<ApiResponse<MarketCalendarResponse>>> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    let calendar = state
        .market_calendar
        .update(year, request)
        .map_err(map_calendar_error)?;
    Ok(Json(ApiResponse::new(
        calendar,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Remover calendário B3 de um ano
///
/// Remove o calendário correspondente ao ano informado do ledger SQLite.
///
/// Retorna confirmação dentro do envelope padrão. A carga automática de 2026 é
/// aplicada uma única vez por ledger, portanto a exclusão permanece depois de
/// reinicializações. Retorna `404` para ano ausente e exige credencial configurada.
#[utoipa::path(
    delete,
    path = "/calendar/{year}",
    tag = "calendário",
    params(("year" = i32, Path, description = "Ano civil do calendário", example = 2026)),
    responses(
        (status = 200, description = "Calendário removido", body = MarketCalendarDeleteApiResponse),
        (status = 404, description = "Não há calendário cadastrado para o ano", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de alterar calendários", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn delete_calendar(
    State(state): State<Arc<AppState>>,
    Path(year): Path<i32>,
) -> AppResult<Json<ApiResponse<MarketCalendarDeleteResponse>>> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    state
        .market_calendar
        .delete(year)
        .map_err(map_calendar_error)?;
    Ok(Json(ApiResponse::new(
        MarketCalendarDeleteResponse {
            ano: year,
            removido: true,
        },
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Consultar horários da plataforma Trader
///
/// Retorna as modalidades e os horários gerais publicados na página de negociação
/// do Trader da B3. Os horários representam o fuso de Brasília.
///
/// Inclui Tela, RFQ e Voice e a fonte utilizada. Horários gerais não substituem
/// horários especiais de datas específicas disponíveis em `/calendar/{year}`.
#[utoipa::path(
    get,
    path = "/negotiation-hours",
    tag = "calendário",
    responses(
        (status = 200, description = "Horários publicados por modalidade", body = NegotiationHoursApiResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn negotiation_hours(
    State(_state): State<Arc<AppState>>,
) -> Json<ApiResponse<NegotiationHoursResponse>> {
    let start = Instant::now();
    Json(ApiResponse::new(
        NegotiationHoursResponse {
            fuso_horario: "America/Sao_Paulo".to_string(),
            fonte: NEGOTIATION_HOURS_SOURCE.to_string(),
            modalidades: vec![
                NegotiationSessionHours {
                    modalidade: "Tela".to_string(),
                    abertura: "09:00".to_string(),
                    fechamento: "18:00".to_string(),
                },
                NegotiationSessionHours {
                    modalidade: "RFQ".to_string(),
                    abertura: "09:00".to_string(),
                    fechamento: "18:00".to_string(),
                },
                NegotiationSessionHours {
                    modalidade: "Voice".to_string(),
                    abertura: "08:00".to_string(),
                    fechamento: "19:15".to_string(),
                },
            ],
        },
        start.elapsed().as_secs_f64() * 1000.0,
    ))
}

/// Build market-calendar routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/calendar", get(list_calendars).post(create_calendar))
        .route(
            "/calendar/:year",
            get(get_calendar)
                .put(update_calendar)
                .delete(delete_calendar),
        )
        .route("/negotiation-hours", get(negotiation_hours))
        .with_state(state)
}
