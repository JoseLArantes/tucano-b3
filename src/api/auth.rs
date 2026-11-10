use super::release_error::ApiError;
use crate::config::B3Settings;
use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub fn is_public(path: &str, method: &Method) -> bool {
    *method == Method::GET
        && (matches!(path, "/healthz" | "/api/v1/health" | "/openapi.json")
            || path == "/docs"
            || path.starts_with("/docs/"))
}

pub async fn require_bearer_token(
    State(settings): State<Arc<B3Settings>>,
    request: Request,
    next: Next,
) -> Response {
    if is_public(request.uri().path(), request.method()) {
        return next.run(request).await;
    }
    if settings.validate_api_credentials().is_err() {
        return ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "AUTH_NOT_CONFIGURED",
            "Autenticação não configurada.",
        )
        .into_response();
    }
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));
    let admin = token == Some(settings.tucano_b3_admin_token.as_str());
    let read = token == Some(settings.tucano_b3_read_token.as_str());
    if !admin && !read {
        return ApiError::new(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "Credencial Bearer ausente ou inválida.",
        )
        .into_response();
    }
    let admin_only = request.method() != Method::GET
        || request.uri().path().starts_with("/api/v1/b3/jobs")
        || request.uri().path().starts_with("/api/v1/b3/sync-runs")
        || request.uri().path().starts_with("/api/v1/b3/quarantine")
        || request
            .uri()
            .path()
            .starts_with("/api/v1/b3/pending-updates");
    let admin_only = admin_only
        || request.uri().path().starts_with("/api/v1/b3/internal/")
        || request
            .uri()
            .path()
            .starts_with("/api/v1/b3/data-revisions")
        || request.uri().path().starts_with("/api/v1/b3/source-files");
    if admin_only && !admin {
        return ApiError::new(
            StatusCode::FORBIDDEN,
            "ADMIN_REQUIRED",
            "A operação exige credencial administrativa.",
        )
        .into_response();
    }
    next.run(request).await
}
