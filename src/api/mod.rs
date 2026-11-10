//! API routes module for tucano-b3.
//!
//! All endpoints are nested under /api/v1/.

pub mod cotahist;
pub mod health;
pub mod ibov;
pub mod info;

use axum::Router;

/// Build all API routes.
pub fn routes() -> Router {
    Router::new().merge(health::routes()).merge(info::routes())
}
