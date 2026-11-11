//! Prometheus metrics endpoint.
//!
//! GET /api/v1/metrics or /metrics
//!
//! Returns metrics in Prometheus exposition format.
//! This endpoint is never cached and never rate-limited.

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use crate::metrics::AppMetrics;

/// Shared state for the metrics endpoint.
#[derive(Clone)]
pub struct MetricsState {
    pub metrics: Arc<AppMetrics>,
    pub start_time: std::time::Instant,
}

/// GET /api/v1/metrics
///
/// Retorna métricas no formato Prometheus.
/// Este endpoint nunca é cacheado e nunca sofre rate limiting.
async fn metrics_handler(State(state): State<MetricsState>) -> impl IntoResponse {
    // Update uptime before rendering
    let uptime = state.start_time.elapsed().as_secs();
    state.metrics.update_uptime(uptime);

    let body = state.metrics.render();

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

/// Build metrics routes.
pub fn routes(metrics: Arc<AppMetrics>, start_time: std::time::Instant) -> Router {
    let state = MetricsState {
        metrics,
        start_time,
    };

    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}
