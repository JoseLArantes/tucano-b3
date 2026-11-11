//! Cache middleware.
//!
//! Intercepts HTTP requests and serves cached responses when available.
//! Only caches GET requests to cacheable paths.

use crate::cache::key::{generate_cache_key, is_cacheable};
use crate::cache::store::CacheStore;
use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Tower Layer for cache middleware.
#[derive(Clone)]
pub struct CacheLayer {
    cache: Arc<CacheStore>,
}

impl CacheLayer {
    pub fn new(cache: Arc<CacheStore>) -> Self {
        Self { cache }
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CacheMiddleware {
            inner,
            cache: Arc::clone(&self.cache),
        }
    }
}

/// Tower Service that checks cache before forwarding to handler.
#[derive(Clone)]
pub struct CacheMiddleware<S> {
    inner: S,
    cache: Arc<CacheStore>,
}

impl<S> Service<Request<Body>> for CacheMiddleware<S>
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
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or("").to_string();

        // Only cache GET requests to cacheable paths when cache is enabled
        if method != "GET" || !is_cacheable(&path) || !self.cache.is_enabled() {
            let mut inner = self.inner.clone();
            return Box::pin(async move { inner.call(req).await });
        }

        let cache_key = generate_cache_key(&method, &path, &query);
        let cache = Arc::clone(&self.cache);

        // Check cache
        if let Some(entry) = cache.get(&cache_key) {
            tracing::debug!(key = %cache_key, path = %path, "Cache HIT");

            let status_code = StatusCode::from_u16(entry.status).unwrap_or(StatusCode::OK);
            let response = Response::builder()
                .status(status_code)
                .header("content-type", &entry.content_type)
                .header("x-cache", "HIT")
                .body(Body::from(entry.body))
                .unwrap_or_else(|_| Response::new(Body::empty()));

            return Box::pin(async move { Ok(response) });
        }

        tracing::debug!(key = %cache_key, path = %path, "Cache MISS");

        // Cache miss — forward to handler and cache the response
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let response = inner.call(req).await?;

            // Only cache successful responses
            if response.status().is_success() {
                let status = response.status().as_u16();
                let content_type = response
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("application/json")
                    .to_string();

                // Read response body for caching
                let (parts, body) = response.into_parts();
                let body_bytes = axum::body::to_bytes(body, usize::MAX)
                    .await
                    .unwrap_or_default();

                // Store in cache
                cache.insert(cache_key, body_bytes.to_vec(), status, content_type.clone());

                // Build response with original body and MISS header
                let mut res = Response::from_parts(parts, Body::from(body_bytes));
                res.headers_mut().insert(
                    axum::http::header::HeaderName::from_static("x-cache"),
                    axum::http::HeaderValue::from_static("MISS"),
                );

                return Ok(res);
            }

            Ok(response)
        })
    }
}
