//! API routes module for tucano-b3.
//!
//! All endpoints are nested under /api/v1/.

pub mod cotahist;
pub mod health;
pub mod ibov;
pub mod info;
pub mod metrics;
pub mod openapi;

use crate::cache::store::CacheStore;
use crate::data::reload::DataLoader;
use axum::Router;
use std::sync::Arc;

/// Build all API routes.
pub fn routes(data_loader: Arc<DataLoader>, cache: Arc<CacheStore>) -> Router {
    let state = Arc::new(health::AppState::new(data_loader, cache));

    Router::new()
        .merge(health::routes(state.clone()))
        .merge(info::routes(state.clone()))
        .merge(cotahist::routes(state.clone()))
        .merge(ibov::routes(state))
}
