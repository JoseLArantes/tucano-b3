//! API routes module for tucano-b3.
//!
//! All endpoints are nested under /api/v1/.

pub mod cotahist;
pub mod health;
pub mod ibov;
pub mod info;

use crate::data::reload::DataLoader;
use axum::Router;
use std::sync::Arc;

/// Build all API routes.
pub fn routes(data_loader: Arc<DataLoader>) -> Router {
    let state = Arc::new(health::AppState::new(data_loader));

    Router::new()
        .merge(health::routes(state.data_loader.clone()))
        .merge(info::routes(state.data_loader.clone()))
        .merge(cotahist::routes(state.clone()))
        .merge(ibov::routes(state))
}
