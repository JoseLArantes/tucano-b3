//! Health check endpoint.
//!
//! GET /api/v1/health
//!
//! Returns service health status for load balancers and monitoring.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;
use utoipa::ToSchema;

/// Shared application state.
///
/// NOTE: In M0, this is minimal. It will be extended in M1+ with
/// DataFusion context, cache, and metrics.
#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub version: String,
}

impl AppState {
    /// Create a new AppState initialized with current time and package version.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Health check response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// GET /api/v1/health
///
/// Returns the health status of the service.
/// Used by load balancers and container orchestrators.
async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(HealthResponse {
        status: crate::locale::pt_br::HEALTH_STATUS_HEALTHY.to_string(),
        version: state.version.clone(),
        uptime_seconds: uptime,
    })
}

/// Build health routes.
pub fn routes() -> Router {
    let state = Arc::new(AppState::new());

    Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
}
