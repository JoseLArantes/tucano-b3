use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Falha tipada sem SQL, caminhos privados ou credenciais.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Código estável da categoria da falha.
    pub code: String,
    /// Explicação segura em português.
    pub message: String,
    /// Identificador para correlação da ocorrência.
    pub request_id: Uuid,
}

pub struct ApiError(pub StatusCode, pub ErrorResponse);
impl ApiError {
    pub fn invalid_path() -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ID",
            "Identificador de recurso inválido.",
        )
    }
    pub fn new(status: StatusCode, code: &str, message: &str) -> Self {
        Self(
            status,
            ErrorResponse {
                code: code.into(),
                message: message.into(),
                request_id: Uuid::new_v4(),
            },
        )
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}
impl From<crate::error::AppError> for ApiError {
    fn from(error: crate::error::AppError) -> Self {
        use crate::error::AppError;
        match error {
            AppError::NotFound(_) => Self::new(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Recurso individual não encontrado.",
            ),
            AppError::Validation(_) | AppError::Layout(_) | AppError::HostNotAllowed(_) => {
                Self::new(
                    StatusCode::BAD_REQUEST,
                    "INVALID_REQUEST",
                    "Pedido inválido para a operação solicitada.",
                )
            }
            AppError::LockConflict(_) => Self::new(
                StatusCode::CONFLICT,
                "CONFLICT",
                "Operação incompatível, cancelada ou concorrente.",
            ),
            _ => Self::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Falha interna; consulte o identificador da ocorrência.",
            ),
        }
    }
}
