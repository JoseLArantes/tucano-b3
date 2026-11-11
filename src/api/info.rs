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
use crate::data::reload::DataLoader;

/// Service info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct InfoResponse {
    pub name: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Obter informações do serviço
///
/// Retorna metadados do serviço, incluindo versão, tempo de atividade e informações sobre os dados carregados.
#[utoipa::path(
    get,
    path = "/info",
    tag = "sistema",
    responses(
        (status = 200, description = "Informações retornadas", body = InfoResponse),
    ),
)]
pub(crate) async fn info_handler(State(state): State<Arc<AppState>>) -> Json<InfoResponse> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(InfoResponse {
        name: "tucano-b3".to_string(),
        version: state.version.clone(),
        uptime_seconds: uptime,
    })
}

/// Build info routes.
pub fn routes(data_loader: Arc<DataLoader>) -> Router {
    let state = Arc::new(AppState::new(data_loader));

    Router::new()
        .route("/info", get(info_handler))
        .with_state(state)
}
