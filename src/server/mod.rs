//! HTTP server module for tucano-b3.
//!
//! Uses axum with tower middleware for routing, compression, caching, rate limiting, and metrics.
//! Supports graceful shutdown via SIGTERM/SIGINT.

use crate::api::openapi::ApiDoc;
use crate::cache::store::CacheStore;
use crate::config::AppConfig;
use crate::data::reload::DataLoader;
use crate::error::AppError;
use crate::metrics::AppMetrics;
use crate::middleware::{
    AuthLayer, CacheLayer, MetricsLayer, RateLimitLayer, RateLimitState, RequestIdLayer,
    RequestLoggingLayer,
};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Build the axum Router with all routes and middleware.
///
/// This function is called once at startup and returns a fully-configured router.
pub fn build_router(
    config: &AppConfig,
    data_loader: Arc<DataLoader>,
    cache: Arc<CacheStore>,
    rate_limit_state: Arc<RateLimitState>,
    metrics: Arc<AppMetrics>,
    start_time: std::time::Instant,
) -> Router {
    let health_state = Arc::new(crate::api::health::AppState::new(
        data_loader.clone(),
        cache.clone(),
    ));

    let request_timeout = Duration::from_secs(config.server.request_timeout_seconds);

    // Query routes keep the short timeout. Ingest is merged beside them so an
    // annual COTAHIST upload is not aborted while the body is still being read.
    let api_routes = crate::api::routes(data_loader, cache.clone())
        .merge(crate::api::metrics::routes(metrics.clone(), start_time))
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ));
    let ingest_routes =
        crate::api::ingest::routes(health_state.clone()).layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            crate::api::ingest::INGEST_REQUEST_TIMEOUT,
        ));

    Router::new()
        // Public health check alias at root
        .route(
            "/health",
            axum::routing::get(crate::api::health::health_handler).with_state(health_state),
        )
        // Standalone metrics endpoint (standard Prometheus scrape /metrics)
        .merge(crate::api::metrics::routes(metrics.clone(), start_time))
        // Swagger UI (also serves /api/docs/openapi.json)
        .merge(
            SwaggerUi::new("/api/docs")
                .url("/api/docs/openapi.json", ApiDoc::openapi())
                .config(utoipa_swagger_ui::Config::default().persist_authorization(true)),
        )
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        // Added after the short timeout, with its own per-router limits.
        .nest("/api/v1", api_routes.merge(ingest_routes))
        .layer(CorsLayer::permissive())
        .layer(CacheLayer::new(cache, metrics.clone()))
        .layer(CompressionLayer::new())
        .layer(RateLimitLayer::new(rate_limit_state, metrics.clone()))
        .layer(AuthLayer::new(config.auth.clone()))
        .layer(RequestLoggingLayer::new())
        .layer(RequestIdLayer::new())
        .layer(MetricsLayer::new(metrics))
}

/// Start the HTTP server and block until shutdown signal.
///
/// Handles SIGTERM and SIGINT for graceful shutdown.
/// All in-flight requests are completed before the process exits.
pub async fn run(
    config: AppConfig,
    data_loader: Arc<DataLoader>,
    cache: Arc<CacheStore>,
    rate_limit_state: Arc<RateLimitState>,
    metrics: Arc<AppMetrics>,
    start_time: std::time::Instant,
) -> Result<(), AppError> {
    let router = build_router(
        &config,
        data_loader,
        cache,
        rate_limit_state,
        metrics,
        start_time,
    );

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid bind address: {}", e)))?;

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "Iniciando servidor tucano-b3"
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to bind: {}", e)))?;

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Server error: {}", e)))?;

    tracing::info!("Servidor finalizado com sucesso");
    Ok(())
}

/// Wait for SIGTERM or SIGINT signal.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            tracing::error!("Failed to install CTRL+C signal handler: {}", err);
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(err) => {
                tracing::error!("Failed to install SIGTERM signal handler: {}", err);
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Recebido sinal SIGINT, iniciando encerramento gracioso...");
        }
        _ = terminate => {
            tracing::info!("Recebido sinal SIGTERM, iniciando encerramento gracioso...");
        }
    }
}
