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

use crate::data::reload::DataLoader;

/// Shared application state.
///
/// Extended in M1 to include the data layer.
#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub version: String,
    pub data_loader: Arc<DataLoader>,
}

impl AppState {
    /// Create a new AppState initialized with current time, package version, and DataLoader.
    pub fn new(data_loader: Arc<DataLoader>) -> Self {
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_loader,
        }
    }
}

/// Health check response.
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub data_files_loaded: usize,
    pub tickers_count: usize,
}

/// Verificar saúde do serviço
///
/// Retorna o status de saúde do serviço. Usado por balanceadores de carga e orquestradores de containers.
#[utoipa::path(
    get,
    path = "/health",
    tag = "sistema",
    responses(
        (status = 200, description = "Serviço saudável", body = HealthResponse),
    ),
)]
pub(crate) async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let query_service = state.data_loader.get_query_service().await;

    Json(HealthResponse {
        status: crate::locale::pt_br::HEALTH_STATUS_HEALTHY.to_string(),
        version: state.version.clone(),
        uptime_seconds: uptime,
        data_files_loaded: query_service.cotahist_file_count()
            + if query_service.has_table("ibov") {
                1
            } else {
                0
            },
        tickers_count: state.data_loader.ticker_registry.count(),
    })
}

/// Build health routes.
pub fn routes(data_loader: Arc<DataLoader>) -> Router {
    let state = Arc::new(AppState::new(data_loader));

    Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
}
