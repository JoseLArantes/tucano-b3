//! API routes module for tucano-b3.
//!
//! All endpoints are nested under /api/v1/.

pub mod cotahist;
pub mod cotahist_analytics;
pub mod health;
pub mod index_metrics;
pub mod info;
pub mod ingest;
pub mod market_calendar;
pub mod metrics;
pub mod openapi;

use crate::cache::store::CacheStore;
use crate::data::reload::DataLoader;
use crate::ingest::index_metrics::IndexIngestor;
use axum::Router;
use std::sync::Arc;

/// Build API routes except ingestion.
///
/// Ingest routes are mounted separately so they can accept large ZIP bodies
/// and a longer request timeout.
pub fn routes(
    data_loader: Arc<DataLoader>,
    cache: Arc<CacheStore>,
    index_ingestor: Arc<IndexIngestor>,
    auth_enabled: bool,
) -> Router {
    let state = Arc::new(health::AppState::new(
        data_loader,
        cache,
        index_ingestor,
        auth_enabled,
    ));

    Router::new()
        .merge(health::routes(state.clone()))
        .merge(info::routes(state.clone()))
        .merge(cotahist::routes(state.clone()))
        .merge(cotahist_analytics::routes(state.clone()))
        .merge(index_metrics::routes(state.clone()))
        .merge(market_calendar::routes(state))
}
