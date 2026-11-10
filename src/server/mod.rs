//! HTTP server module for tucano-b3.
//!
//! Uses axum with tower middleware for routing, compression, and tracing.
//! Supports graceful shutdown via SIGTERM/SIGINT.

use crate::config::AppConfig;
use crate::data::reload::DataLoader;
use crate::error::AppError;
use crate::middleware::{RateLimitLayer, RateLimitState, RequestIdLayer};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;

/// Build the axum Router with all routes and middleware.
///
/// This function is called once at startup and returns a fully-configured router.
pub fn build_router(
    config: &AppConfig,
    data_loader: Arc<DataLoader>,
    rate_limit_state: Arc<RateLimitState>,
) -> Router {
    let api_routes = crate::api::routes(data_loader);

    Router::new()
        .nest("/api/v1", api_routes)
        .layer(RateLimitLayer::new(rate_limit_state))
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(RequestIdLayer::new())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.server.request_timeout_seconds),
        ))
}

/// Start the HTTP server and block until shutdown signal.
///
/// Handles SIGTERM and SIGINT for graceful shutdown.
/// All in-flight requests are completed before the process exits.
pub async fn run(
    config: AppConfig,
    data_loader: Arc<DataLoader>,
    rate_limit_state: Arc<RateLimitState>,
) -> Result<(), AppError> {
    let router = build_router(&config, data_loader, rate_limit_state);

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

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Server error: {}", e)))?;

    tracing::info!("Servidor tucano-b3 finalizado com sucesso");

    Ok(())
}

/// Wait for a shutdown signal (SIGTERM or SIGINT/Ctrl+C).
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = signal::ctrl_c().await {
            tracing::error!("Failed to install Ctrl+C handler: {e}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!("Failed to install SIGTERM handler: {e}");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Recebido SIGINT, iniciando shutdown gracioso");
        }
        _ = terminate => {
            tracing::info!("Recebido SIGTERM, iniciando shutdown gracioso");
        }
    }
}
