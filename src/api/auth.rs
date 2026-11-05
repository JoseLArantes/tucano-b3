use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::config::B3Settings;

#[derive(Clone)]
pub struct AuthState {
    pub token: String,
}

pub async fn require_bearer_token(
    State(settings): State<Arc<B3Settings>>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();

    // Whitelist public endpoints
    if path == "/healthz"
        || path == "/api/v1/health"
        || path == "/api/v1/b3/worker/health"
        || path == "/docs"
        || path == "/redoc"
        || path == "/openapi.json"
        || path.starts_with("/swagger-ui")
    {
        return next.run(req).await;
    }

    let auth_header = req.headers().get(axum::http::header::AUTHORIZATION);
    let is_valid = match auth_header {
        Some(val) => {
            if let Ok(s) = val.to_str() {
                if let Some(token) = s.strip_prefix("Bearer ") {
                    token.trim() == settings.tucano_b3_token
                } else {
                    false
                }
            } else {
                false
            }
        }
        None => false,
    };

    if !is_valid {
        let body = Json(json!({
            "error": "Unauthorized",
            "message": "Token de autenticação ausente ou inválido. Use 'Authorization: Bearer <token>'."
        }));
        return (StatusCode::UNAUTHORIZED, body).into_response();
    }

    next.run(req).await
}

