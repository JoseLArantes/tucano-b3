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

use crate::cache::store::CacheStore;
use crate::config::ObjectStorageConfig;
use crate::data::reload::DataLoader;
use crate::ingest::index_metrics::IndexIngestor;
use crate::services::market_calendar::MarketCalendarService;
use crate::services::proventos::ProventosService;

/// Shared application state.
///
/// Extended in M1 to include the data layer and in M4 to include the cache store.
#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub version: String,
    pub data_loader: Arc<DataLoader>,
    pub cache: Arc<CacheStore>,
    /// S3-compatible bucket used by `POST /api/v1/ingest/bucket`.
    pub object_storage: ObjectStorageConfig,
    pub index_ingestor: Arc<IndexIngestor>,
    pub market_calendar: Arc<MarketCalendarService>,
    pub proventos: Arc<ProventosService>,
    pub auth_enabled: bool,
}

impl AppState {
    /// Create a new AppState initialized with current time, package version, DataLoader, and CacheStore.
    pub fn new(
        data_loader: Arc<DataLoader>,
        cache: Arc<CacheStore>,
        index_ingestor: Arc<IndexIngestor>,
        auth_enabled: bool,
    ) -> Self {
        let market_calendar = Arc::new(
            MarketCalendarService::new(index_ingestor.ledger())
                .expect("B3 market calendar ledger must be available"),
        );
        let proventos = Arc::new(
            ProventosService::new(index_ingestor.ledger())
                .expect("proventos ledger and seed must be available"),
        );
        Self {
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_loader,
            cache,
            object_storage: ObjectStorageConfig::default(),
            index_ingestor,
            market_calendar,
            proventos,
            auth_enabled,
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
    pub cache_entries: usize,
    pub cache_enabled: bool,
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
        cache_entries: state.cache.len(),
        cache_enabled: state.cache.is_enabled(),
    })
}

/// Build health routes.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .with_state(state)
}
