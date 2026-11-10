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
    Router::new()
        .merge(health::routes(data_loader.clone()))
        .merge(info::routes(data_loader))
}
