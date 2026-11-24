//! CRUD and reporting endpoints for registered cash distributions.

pub(crate) mod reports;
pub(crate) mod upload;

use crate::api::health::AppState;
#[allow(unused_imports)]
use crate::error::ErrorResponse;
use crate::error::{AppError, AppResult};
use crate::models::envelope::ApiResponse;
#[allow(unused_imports)]
use crate::models::envelope::{
    ProventoApiResponse, ProventoDeleteApiResponse, ProventosListApiResponse,
};
use crate::models::proventos::{
    ProventoDeleteResponse, ProventoRequest, ProventoResponse, ProventosQuery,
};
use crate::services::proventos::ProventosError;
use axum::extract::{DefaultBodyLimit, FromRequest, FromRequestParts, Path, Query, Request, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Instant;

/// JSON extractor that keeps malformed write requests in the API error envelope.
pub struct ProventosJson<T>(T);

/// Query extractor that returns malformed filters in the standard error envelope.
pub struct ProventosQueryParams<T>(T);

#[axum::async_trait]
impl<S, T> FromRequestParts<S> for ProventosQueryParams<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(|error| AppError::InvalidParam {
                message: format!("Parâmetros de consulta inválidos: {error}"),
            })
    }
}

#[axum::async_trait]
impl<S, T> FromRequest<S> for ProventosJson<T>
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

fn map_proventos_error(error: ProventosError) -> AppError {
    match error {
        ProventosError::NotFound { id } => AppError::ResourceNotFound {
            resource: format!("provento {id}"),
        },
        ProventosError::Invalid(message) => AppError::InvalidParam { message },
        error @ (ProventosError::Ledger(_) | ProventosError::Serialization(_)) => {
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

/// Listar proventos cadastrados
///
/// Retorna registros em ordem determinística por data de pagamento, ticker, tipo
/// e identificador. Use `ticker` para um código ou `tickers` para uma lista de até
/// 100 códigos separados por vírgula; esses parâmetros são mutuamente exclusivos.
/// A lista de tickers é combinada com `tipo` e o intervalo inclusivo de pagamento;
/// registros sem data não entram no intervalo. `pagina` começa em 1 e `por_pagina`
/// assume 50, limitado a 1–1000. Valores são decimais textuais exatos em reais,
/// com seis casas. `obs` preserva a fonte e as observações fornecidas, sem validação
/// ou classificação como série oficial B3.
#[utoipa::path(
    get,
    path = "/proventos",
    tag = "proventos",
    params(ProventosQuery),
    responses(
        (status = 200, description = "Página de proventos cadastrados", body = ProventosListApiResponse),
        (status = 400, description = "Filtro ou intervalo de datas inválido", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn list_proventos(
    State(state): State<Arc<AppState>>,
    ProventosQueryParams(query): ProventosQueryParams<ProventosQuery>,
) -> AppResult<Json<ApiResponse<crate::models::proventos::ProventosListResponse>>> {
    let start = Instant::now();
    let list = state.proventos.list(query).map_err(map_proventos_error)?;
    Ok(Json(ApiResponse::new(
        list,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Consultar um provento por identificador
///
/// Retorna o valor exato, as datas disponíveis, as observações de origem e os
/// instantes de criação e atualização. O identificador não muda após alterações.
#[utoipa::path(
    get,
    path = "/proventos/{id}",
    tag = "proventos",
    params(("id" = i64, Path, description = "Identificador estável do provento", example = 23)),
    responses(
        (status = 200, description = "Provento encontrado", body = ProventoApiResponse),
        (status = 404, description = "Provento não encontrado", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn get_provento(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProventoResponse>>> {
    let start = Instant::now();
    let record = state.proventos.get(id).map_err(map_proventos_error)?;
    Ok(Json(ApiResponse::new(
        record,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Cadastrar provento
///
/// Cria um registro persistido no ledger SQLite. `tipo` aceita `Dividendo`,
/// `JCP` ou `Rendimento`; `valor` deve ser enviado como string decimal positiva
/// em reais, com até seis casas, sem arredondamento. Datas usam `AAAA-MM-DD` e
/// podem ser nulas quando a fonte não as informa. `obs` deve identificar a fonte
/// quando conhecida e pode conter até 255 caracteres. O ticker é normalizado para
/// maiúsculas. Cada chamada cria um novo ID, mesmo que os campos coincidam com
/// outro lançamento. Escritas exigem autenticação configurada.
#[utoipa::path(
    post,
    path = "/proventos",
    tag = "proventos",
    request_body = ProventoRequest,
    responses(
        (status = 201, description = "Provento criado", body = ProventoApiResponse),
        (status = 400, description = "Dados do provento inválidos", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de cadastrar proventos", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn create_provento(
    State(state): State<Arc<AppState>>,
    ProventosJson(request): ProventosJson<ProventoRequest>,
) -> AppResult<(StatusCode, Json<ApiResponse<ProventoResponse>>)> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    let record = state
        .proventos
        .create(request)
        .map_err(map_proventos_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::new(
            record,
            start.elapsed().as_secs_f64() * 1000.0,
        )),
    ))
}

/// Substituir provento
///
/// Substitui todos os campos de negócio do registro indicado e mantém o mesmo
/// identificador e instante de criação. Os mesmos limites de tipo, valor, datas,
/// ticker e observação aplicados em `POST` também são aplicados aqui.
#[utoipa::path(
    put,
    path = "/proventos/{id}",
    tag = "proventos",
    params(("id" = i64, Path, description = "Identificador estável do provento", example = 23)),
    request_body = ProventoRequest,
    responses(
        (status = 200, description = "Provento atualizado", body = ProventoApiResponse),
        (status = 400, description = "Dados do provento inválidos", body = ErrorResponse),
        (status = 404, description = "Provento não encontrado", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de alterar proventos", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn update_provento(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    ProventosJson(request): ProventosJson<ProventoRequest>,
) -> AppResult<Json<ApiResponse<ProventoResponse>>> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    let record = state
        .proventos
        .update(id, request)
        .map_err(map_proventos_error)?;
    Ok(Json(ApiResponse::new(
        record,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Excluir provento
///
/// Remove o registro pelo identificador e retorna confirmação no envelope padrão.
/// A exclusão é persistida no ledger SQLite e exige autenticação configurada.
#[utoipa::path(
    delete,
    path = "/proventos/{id}",
    tag = "proventos",
    params(("id" = i64, Path, description = "Identificador estável do provento", example = 23)),
    responses(
        (status = 200, description = "Provento removido", body = ProventoDeleteApiResponse),
        (status = 404, description = "Provento não encontrado", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de excluir proventos", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn delete_provento(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ProventoDeleteResponse>>> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    let result = state.proventos.delete(id).map_err(map_proventos_error)?;
    Ok(Json(ApiResponse::new(
        result,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

/// Build authenticated provento CRUD routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/proventos", get(list_proventos).post(create_provento))
        .route(
            "/proventos/upload",
            post(upload::upload_csv).layer(DefaultBodyLimit::max(upload::UPLOAD_BODY_LIMIT)),
        )
        .route("/proventos/yearly", get(reports::yearly))
        .route("/proventos/period", get(reports::period))
        .route("/proventos/series", get(reports::series))
        .route(
            "/proventos/:id",
            get(get_provento)
                .put(update_provento)
                .delete(delete_provento),
        )
        .with_state(state)
}
