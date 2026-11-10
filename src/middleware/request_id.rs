//! Request ID middleware.
//!
//! Assigns a UUID v4 to every incoming request.
//! The ID is added to the response as `X-Request-Id` header.
//! The ID is also available to handlers via request extensions.

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use uuid::Uuid;

/// Header name for the request ID.
pub const REQUEST_ID_HEADER: &str = "X-Request-Id";

/// Tower Layer that adds request ID generation.
#[derive(Clone, Debug, Default)]
pub struct RequestIdLayer;

impl RequestIdLayer {
    /// Create a new RequestIdLayer.
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdMiddleware { inner }
    }
}

/// Tower Service that generates and propagates request IDs.
#[derive(Clone, Debug)]
pub struct RequestIdMiddleware<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for RequestIdMiddleware<S>
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

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // Generate or extract request ID
        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Insert into request extensions for downstream handlers
        req.extensions_mut().insert(RequestId(request_id.clone()));

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(req).await?;

            // Add request ID to response headers
            if let Ok(header_val) = HeaderValue::from_str(&request_id) {
                response.headers_mut().insert(REQUEST_ID_HEADER, header_val);
            }

            Ok(response)
        })
    }
}

/// Extension type for accessing request ID in handlers.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);
