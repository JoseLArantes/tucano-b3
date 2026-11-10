//! Service info endpoint.
//!
//! GET /api/v1/info
//!
//! Returns service metadata and data range information.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

use super::health::AppState;

/// Service info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// GET /api/v1/info
async fn info_handler(State(state): State<Arc<AppState>>) -> Json<InfoResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(InfoResponse {
        name: "tucano-b3".to_string(),
        version: state.version.clone(),
        uptime_seconds: uptime,
    })
}

/// Build info routes.
pub fn routes() -> Router {
    let state = Arc::new(AppState::new());

    Router::new()
        .route("/info", get(info_handler))
        .with_state(state)
}
