//! Authentication middleware for tucano-b3.
//!
//! Validates Bearer token or API key against configured tokens.
//! Public endpoints (/health, /api/v1/health, /redoc, /api/docs) bypass authentication.

use axum::body::Body;
use axum::http::header::{self, HeaderName, HeaderValue};
use axum::http::{Request, Response, StatusCode};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use crate::config::AuthConfig;

/// Header names for API keys and tokens
pub const API_KEY_HEADER: &str = "X-API-Key";
pub const TOKEN_HEADER: &str = "X-Token";

/// Check if a path is public and does not require authentication.
pub fn is_public_path(path: &str) -> bool {
    path == "/health"
        || path == "/api/v1/health"
        || path == "/redoc"
        || path == "/api/redoc"
        || path == "/api/docs"
        || path.starts_with("/api/docs/")
        || path == "/openapi.json"
}

/// Tower Layer for token authentication.
#[derive(Clone)]
pub struct AuthLayer {
    config: Arc<AuthConfig>,
}

impl AuthLayer {
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            config: Arc::clone(&self.config),
        }
    }
}

/// Tower Service that enforces authentication.
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    config: Arc<AuthConfig>,
}

impl<S> AuthMiddleware<S> {
    /// Extract authentication token from request headers.
    ///
    /// Checks:
    /// 1. `Authorization: Bearer <token>` or `Authorization: <token>`
    /// 2. `X-API-Key: <token>`
    /// 3. `X-Token: <token>`
    fn extract_token<'a>(&self, req: &'a Request<Body>) -> Option<&'a str> {
        // 1. Authorization header
        if let Some(auth_val) = req.headers().get(header::AUTHORIZATION) {
            if let Ok(auth_str) = auth_val.to_str() {
                let auth_str = auth_str.trim();
                if let Some(token) = auth_str.strip_prefix("Bearer ") {
                    return Some(token.trim());
                } else if let Some(token) = auth_str.strip_prefix("bearer ") {
                    return Some(token.trim());
                } else if !auth_str.is_empty() {
                    return Some(auth_str);
                }
            }
        }

        // 2. X-API-Key header
        if let Some(key_val) = req.headers().get(API_KEY_HEADER) {
            if let Ok(key_str) = key_val.to_str() {
                let key_str = key_str.trim();
                if !key_str.is_empty() {
                    return Some(key_str);
                }
            }
        }

        // 3. X-Token header
        if let Some(token_val) = req.headers().get(TOKEN_HEADER) {
            if let Ok(token_str) = token_val.to_str() {
                let token_str = token_str.trim();
                if !token_str.is_empty() {
                    return Some(token_str);
                }
            }
        }

        None
    }
}

impl<S> Service<Request<Body>> for AuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let path = req.uri().path().to_string();

        // If authentication is disabled or path is public, proceed
        if !self.config.is_enabled() || is_public_path(&path) {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        // Check token
        let token = self.extract_token(&req);
        let is_valid = match token {
            Some(t) => self.config.validate_token(t),
            None => false,
        };

        if is_valid {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        // Unauthorized
        tracing::warn!(path = %path, "Requisição rejeitada: token ausente ou inválido");

        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let body = json!({
            "error": {
                "code": "UNAUTHORIZED",
                "message": "Não autorizado",
                "details": "Forneça um token válido via cabeçalho 'Authorization: Bearer <token>' ou 'X-API-Key'.",
                "request_id": request_id,
            }
        });

        let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
        let response = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::CONTENT_TYPE, "application/json")
            .header(
                header::WWW_AUTHENTICATE,
                "Bearer realm=\"tucano-b3\", charset=\"UTF-8\"",
            )
            .header(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str(&request_id).unwrap_or(HeaderValue::from_static("unknown")),
            )
            .body(Body::from(body_bytes))
            .unwrap_or_else(|_| Response::new(Body::empty()));

        Box::pin(async move { Ok(response) })
    }
}
