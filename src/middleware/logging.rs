//! Request logging middleware.
//!
//! Logs every request with structured context:
//! - request_id
//! - method
//! - path
//! - status code
//! - duration_ms
//! - cached (true/false)

use axum::body::Body;
use axum::http::Request;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

/// Tower Layer for request logging.
#[derive(Clone, Debug, Default)]
pub struct RequestLoggingLayer;

impl RequestLoggingLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for RequestLoggingLayer {
    type Service = RequestLoggingMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestLoggingMiddleware { inner }
    }
}

/// Tower Service that logs each request.
#[derive(Clone, Debug)]
pub struct RequestLoggingMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RequestLoggingMiddleware<S>
where
    S: Service<Request<Body>, Response = axum::response::Response> + Clone + Send + 'static,
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
        let request_id = req
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        let start = Instant::now();

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let response = inner.call(req).await?;

            let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
            let status = response.status().as_u16();
            let cached = response
                .headers()
                .get("x-cache")
                .and_then(|v| v.to_str().ok())
                .map(|v| v == "HIT")
                .unwrap_or(false);

            if status >= 500 {
                tracing::error!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = status,
                    duration_ms = %format!("{:.2}", duration_ms),
                    cached = cached,
                    "Requisição processada"
                );
            } else if status >= 400 {
                tracing::warn!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = status,
                    duration_ms = %format!("{:.2}", duration_ms),
                    cached = cached,
                    "Requisição processada"
                );
            } else {
                tracing::info!(
                    request_id = %request_id,
                    method = %method,
                    path = %path,
                    status = status,
                    duration_ms = %format!("{:.2}", duration_ms),
                    cached = cached,
                    "Requisição processada"
                );
            }

            Ok(response)
        })
    }
}
