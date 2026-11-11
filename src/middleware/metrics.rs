//! Request metrics middleware.
//!
//! Records request count and duration for every HTTP request.
//! Labels: method, path (normalized), status code.

use crate::metrics::AppMetrics;
use axum::body::Body;
use axum::http::Request;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;
use tower::{Layer, Service};

/// Tower Layer for request metrics.
#[derive(Clone)]
pub struct MetricsLayer {
    metrics: Arc<AppMetrics>,
}

impl MetricsLayer {
    pub fn new(metrics: Arc<AppMetrics>) -> Self {
        Self { metrics }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsMiddleware {
            inner,
            metrics: Arc::clone(&self.metrics),
        }
    }
}

/// Tower Service that records request metrics.
#[derive(Clone)]
pub struct MetricsMiddleware<S> {
    inner: S,
    metrics: Arc<AppMetrics>,
}

impl<S> Service<Request<Body>> for MetricsMiddleware<S>
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
        let path = normalize_path(req.uri().path());
        let start = Instant::now();

        self.metrics.http_requests_in_flight.inc();

        let mut inner = self.inner.clone();
        let metrics = Arc::clone(&self.metrics);

        Box::pin(async move {
            let response = inner.call(req).await?;

            let duration = start.elapsed().as_secs_f64();
            let status = response.status().as_u16().to_string();

            metrics.http_requests_in_flight.dec();
            metrics
                .http_requests_total
                .with_label_values(&[&method, &path, &status])
                .inc();
            metrics
                .http_request_duration_seconds
                .with_label_values(&[&method, &path])
                .observe(duration);

            Ok(response)
        })
    }
}

/// Normalize URL path for metric labels.
///
/// Replaces dynamic segments with placeholders to prevent label explosion.
/// Examples:
///   /api/v1/cotahist/tickers/VALE3 → /api/v1/cotahist/tickers/{ticker}
///   /api/v1/cotahist/dates/2026-01-01/quotes → /api/v1/cotahist/dates/{date}/quotes
pub fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let mut normalized = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            continue;
        }

        // Replace known dynamic segments
        let is_ticker = i > 0 && segments[i - 1] == "tickers" && !segment.is_empty();
        let is_date = i > 0 && segments[i - 1] == "dates" && !segment.is_empty();

        if is_ticker {
            normalized.push("{ticker}".to_string());
        } else if is_date {
            normalized.push("{date}".to_string());
        } else {
            normalized.push(segment.to_string());
        }
    }

    format!("/{}", normalized.join("/"))
}
