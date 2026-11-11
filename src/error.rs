//! Error handling for tucano-b3.
//!
//! All user-facing error messages are in Brazilian Portuguese.
//! Error codes are stable English identifiers for programmatic handling.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

/// Top-level error type for the tucano-b3 service.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Ticker não encontrado: {ticker}")]
    TickerNotFound { ticker: String },

    #[error("Formato de data inválido: {value}")]
    InvalidDate { value: String },

    #[error("Intervalo de datas inválido")]
    DateRangeInvalid,

    #[error("Nenhum dado encontrado no período")]
    NoDataInRange,

    #[error("Limite de requisições excedido")]
    RateLimited,

    #[error("Erro interno do servidor")]
    Internal(#[from] anyhow::Error),

    #[error("Serviço indisponível")]
    ServiceUnavailable,

    #[error("Parâmetro inválido: {message}")]
    InvalidParam { message: String },
}

impl AppError {
    /// Return the stable programmatic error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TickerNotFound { .. } => "TICKER_NOT_FOUND",
            Self::InvalidDate { .. } => "INVALID_DATE",
            Self::DateRangeInvalid => "DATE_RANGE_INVALID",
            Self::NoDataInRange => "NO_DATA_IN_RANGE",
            Self::RateLimited => "RATE_LIMITED",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            Self::InvalidParam { .. } => "INVALID_PARAM",
        }
    }

    /// Return the corresponding HTTP status code.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::TickerNotFound { .. } => StatusCode::NOT_FOUND,
            Self::InvalidDate { .. } => StatusCode::BAD_REQUEST,
            Self::DateRangeInvalid => StatusCode::BAD_REQUEST,
            Self::NoDataInRange => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::InvalidParam { .. } => StatusCode::BAD_REQUEST,
        }
    }

    /// Return optional contextual details or hints for the user in Brazilian Portuguese.
    pub fn details(&self) -> Option<&'static str> {
        match self {
            Self::TickerNotFound { .. } => {
                Some("Verifique se o ticker existe nos dados ingeridos.")
            }
            Self::InvalidDate { .. } => Some("Use o formato AAAA-MM-DD (ex: 2026-11-10)."),
            Self::DateRangeInvalid => Some("A data inicial não pode ser posterior à data final."),
            Self::NoDataInRange => Some("Verifique o intervalo de datas disponível nos dados."),
            Self::RateLimited => Some("Aguarde e tente novamente."),
            _ => None,
        }
    }
}

/// JSON error response body.
#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

/// Details of the error contained in ErrorResponse.
#[derive(Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Código do erro (identificador estável em inglês)
    pub code: String,
    /// Mensagem de erro em português brasileiro
    pub message: String,
    /// Detalhes adicionais sobre o erro
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Identificador único da requisição
    pub request_id: String,
}

/// Convert AppError into an HTTP response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let request_id = uuid::Uuid::new_v4().to_string();

        let body = ErrorResponse {
            error: ErrorDetail {
                code: self.code().to_string(),
                message: self.to_string(),
                details: self.details().map(|s| s.to_string()),
                request_id,
            },
        };

        (status, Json(body)).into_response()
    }
}

/// Type alias for Result with AppError.
pub type AppResult<T> = Result<T, AppError>;
