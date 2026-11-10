//! Rate limit middleware layer.
//!
//! Tower middleware that enforces per-client rate limits.

use super::bucket::TokenBucket;
use super::keys::ApiKeyRegistry;
use crate::config::RateLimitConfig;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use dashmap::DashMap;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Header for API key authentication.
pub const API_KEY_HEADER: &str = "X-API-Key";

/// Header indicating rate limit remaining.
pub const RATE_LIMIT_REMAINING: &str = "X-RateLimit-Remaining";

/// Header indicating rate limit total.
pub const RATE_LIMIT_LIMIT: &str = "X-RateLimit-Limit";

/// Header indicating seconds to wait before retry.
pub const RETRY_AFTER: &str = "Retry-After";

/// Shared rate limit state.
///
/// Manages per-client token buckets.
/// Thread-safe via DashMap.
pub struct RateLimitState {
    /// Per-client token buckets, keyed by client identity
    buckets: DashMap<String, TokenBucket>,
    /// API key registry
    api_keys: Arc<ApiKeyRegistry>,
    /// Configuration
    config: RateLimitConfig,
}

impl RateLimitState {
    pub fn new(api_keys: Arc<ApiKeyRegistry>, config: RateLimitConfig) -> Self {
        Self {
            buckets: DashMap::new(),
            api_keys,
            config,
        }
    }

    /// Identify the client from the request.
    ///
    /// Priority:
    /// 1. API key from X-API-Key header
    /// 2. Client IP address
    pub fn identify_client(&self, req: &Request<Body>) -> (String, u32) {
        // Check for API key
        if let Some(key_header) = req.headers().get(API_KEY_HEADER) {
            if let Ok(key) = key_header.to_str() {
                let key = key.trim();
                if !key.is_empty() {
                    if let Some(entry) = self.api_keys.validate(key) {
                        let limit = entry
                            .rate_limit
                            .unwrap_or(self.config.api_key_requests_per_minute);
                        tracing::debug!(key_name = %entry.name, "Cliente identificado por API key");
                        return (format!("key:{}", key), limit);
                    }
                    tracing::debug!("API key inválida, usando IP");
                }
            }
        }

        // Fall back to IP
        let ip = req
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        (
            format!("ip:{}", ip),
            self.config.default_requests_per_minute,
        )
    }

    /// Try to acquire a token for the given client.
    ///
    /// Returns Ok(remaining_tokens) or Err(wait_seconds).
    pub fn try_acquire(&self, client_id: &str, limit: u32) -> Result<u32, f64> {
        let mut bucket = self
            .buckets
            .entry(client_id.to_string())
            .or_insert_with(|| TokenBucket::new(limit));

        match bucket.try_acquire() {
            Ok(()) => Ok(bucket.available_tokens() as u32),
            Err(wait) => Err(wait),
        }
    }

    /// Clean up buckets that haven't been used recently.
    ///
    /// Prevents unbounded memory growth from many unique IPs.
    pub fn cleanup(&self, max_buckets: usize) {
        if self.buckets.len() <= max_buckets {
            return;
        }

        // Remove buckets with very few tokens remaining (likely inactive)
        let to_remove: Vec<String> = self
            .buckets
            .iter()
            .filter(|entry| entry.value().available_tokens() < 1.0)
            .map(|entry| entry.key().clone())
            .collect();

        for key in to_remove {
            self.buckets.remove(&key);
        }
    }
}

/// Tower Layer for rate limiting.
#[derive(Clone)]
pub struct RateLimitLayer {
    state: Arc<RateLimitState>,
}

impl RateLimitLayer {
    pub fn new(state: Arc<RateLimitState>) -> Self {
        Self { state }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            state: Arc::clone(&self.state),
        }
    }
}

/// Tower Service that enforces rate limits.
#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    state: Arc<RateLimitState>,
}

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
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

        // Skip rate limiting if disabled
        if !self.state.config.enabled {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        // Never rate-limit health endpoint or metrics endpoint
        if path == "/api/v1/health" || path == "/api/v1/metrics" {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        // Identify client
        let (client_id, limit) = self.state.identify_client(&req);

        // Try to acquire token
        match self.state.try_acquire(&client_id, limit) {
            Ok(remaining) => {
                let mut inner = self.inner.clone();
                Box::pin(async move {
                    let mut response = inner.call(req).await?;

                    // Add rate limit headers safely without unwrap/expect
                    if let Ok(name) = HeaderName::from_lowercase(b"x-ratelimit-remaining") {
                        if let Ok(val) = HeaderValue::from_str(&remaining.to_string()) {
                            response.headers_mut().insert(name, val);
                        }
                    }
                    if let Ok(name) = HeaderName::from_lowercase(b"x-ratelimit-limit") {
                        if let Ok(val) = HeaderValue::from_str(&limit.to_string()) {
                            response.headers_mut().insert(name, val);
                        }
                    }

                    Ok(response)
                })
            }
            Err(wait_seconds) => {
                tracing::warn!(
                    client = %client_id,
                    path = %path,
                    wait = %wait_seconds,
                    "Limite de requisições excedido"
                );

                let retry_after = wait_seconds.ceil() as u64;
                let request_id = uuid::Uuid::new_v4().to_string();
                let body = json!({
                    "error": {
                        "code": "RATE_LIMITED",
                        "message": "Limite de requisições excedido",
                        "details": format!("Aguarde {} segundo(s) e tente novamente.", retry_after),
                        "request_id": request_id
                    }
                });

                let body_str = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
                let mut response = Response::new(Body::from(body_str));
                *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;

                let val = HeaderValue::from_static("application/json");
                response
                    .headers_mut()
                    .insert(axum::http::header::CONTENT_TYPE, val);
                if let Ok(name) = HeaderName::from_lowercase(b"retry-after") {
                    if let Ok(val) = HeaderValue::from_str(&retry_after.to_string()) {
                        response.headers_mut().insert(name, val);
                    }
                }

                Box::pin(async move { Ok(response) })
            }
        }
    }
}
