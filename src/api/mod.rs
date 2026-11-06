pub mod auth;
pub mod release;
pub mod release_error;
pub mod release_extra;
pub mod release_ops;
pub mod routers;
pub mod schemas;

use crate::{
    config::B3Settings,
    services::{query::*, sync_service::B3SyncService},
};
use axum::{
    middleware,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};

pub struct AppState {
    pub pool: PgPool,
    pub settings: Arc<B3Settings>,
    pub sync_service: Arc<B3SyncService>,
}

struct SecurityAddon;
impl Modify for SecurityAddon {
    fn modify(&self, api: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = api.components.as_mut() {
            components.add_security_scheme("bearer_auth",SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).description(Some("Credencial distinta de leitura ou administração; operações de escrita exigem administração.")).build()));
        }
    }
}

#[derive(OpenApi)]
#[openapi(info(title="Tucano B3",version="0.0.1",description="Histórico oficial nominal B3 com precisão, revisão fixada e proveniência. Preços COTAHIST não ajustados por inflação ou eventos corporativos."),modifiers(&SecurityAddon),paths(release::quotes,release::series,release::instruments,release::instrument,release::instrument_identifiers,release::tradable_securities,release::submit_job,release::job,release::cancel_job,release_ops::source_files,release_ops::pending,release_ops::revision,release_ops::conflicts,release_ops::rejections,release_ops::members,release_ops::scan,release_ops::jobs,release_ops::sync_runs,release_ops::capabilities,healthz,health),components(schemas(QuoteFilters,QuotePage,HistoricalQuote,QuoteProvenance,QuoteIdentity,QuoteInternal,Coverage,CoverageStatus,IdentityStatus,InstrumentFilters,InstrumentPage,InstrumentEvidence,InstrumentIdentifierEvidence,InstrumentIdentifierPage,InstrumentIdentifierProvenance,JobView,JobStatus,RevisionView,ConflictView,RejectionView,SourceFileView,MemberView,PendingView,ListFilters,Capability,CapabilityStatus,release::SubmitJob,release::AcceptedJob,release_error::ErrorResponse,Health)),tags((name="COTAHIST",description="Arquivos oficiais COTAHIST anual/mensal/diário; preços nominais sem ajustes"),(name="Cadastro",description="Fotografias oficiais com evidências temporais"),(name="Operação",description="Comandos rastreados, somente para administração"),(name="Saúde",description="Exceções públicas explícitas")))]
struct BaseDoc;

#[derive(OpenApi)]
#[openapi(
    paths(
        release_extra::retry,
        release_extra::diagnostics,
        release_extra::sync_run,
        release_ops::acknowledge_pending
    ),
    components(schemas(
        release_extra::RetryRequest,
        DatasetDiagnostics,
        DiagnosticsFilters,
        release_ops::AcknowledgePending,
        release_ops::PendingAcknowledged
    ))
)]
struct AdditionalDoc;

/// Contrato único composto a partir dos módulos de consulta e operação.
pub struct ApiDoc;
impl OpenApi for ApiDoc {
    fn openapi() -> utoipa::openapi::OpenApi {
        let mut doc = BaseDoc::openapi();
        doc.merge(AdditionalDoc::openapi());
        doc
    }
}

/// Saúde mínima sem nomes de hosts, SQL ou detalhes privados.
#[derive(Serialize, ToSchema)]
pub struct Health {
    /// Disponibilidade do processo/dependência consultada.
    pub healthy: bool,
}
#[utoipa::path(get,path="/healthz",tag="Saúde",summary="Verificar liveness do processo HTTP",description="Exceção pública de liveness, independente da cobertura de dados B3.\nNão dispara coleta, parsing ou manutenção e não representa readiness do banco.\nNão revela credenciais nem detalhes operacionais.",responses((status=200,description="Processo disponível",body=Health),(status=400,description="Não aplicável"),(status=401,description="Exceção pública"),(status=404,description="Não aplicável"),(status=409,description="Não aplicável"),(status=500,description="Falha interna")))]
pub async fn healthz() -> Json<Health> {
    Json(Health { healthy: true })
}
#[utoipa::path(get,path="/api/v1/health",tag="Saúde",summary="Verificar readiness da conexão de dados",description="Exceção pública que consulta a dependência PostgreSQL, sem interpretar arquivos.\nUma base vazia pode estar pronta para operações, mas isso não significa cobertura histórica completa.\nFalhas retornam 503 sem SQL, hosts ou credenciais.",responses((status=200,description="Dependência acessível",body=Health),(status=400,description="Não aplicável"),(status=401,description="Exceção pública"),(status=404,description="Não aplicável"),(status=409,description="Não aplicável"),(status=500,description="Falha interna"),(status=503,description="Banco indisponível")))]
pub async fn health(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Result<Json<Health>, release_error::ApiError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(|_| {
            release_error::ApiError::new(
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "DATABASE_UNAVAILABLE",
                "Dependência indisponível.",
            )
        })?;
    Ok(Json(Health { healthy: true }))
}
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
pub async fn swagger_ui() -> Html<&'static str> {
    Html("<!doctype html><html lang=\"pt-BR\"><title>Tucano B3</title><h1>Contrato Tucano B3 0.0.1</h1><p>Preços históricos nominais sem ajustes.</p><a href=\"/openapi.json\">OpenAPI 3.1</a></html>")
}
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/v1/health", get(health))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
        .route("/docs/", get(swagger_ui))
        .route("/api/v1/b3/cotahist/quotes", get(release::quotes))
        .route("/api/v1/b3/instruments", get(release::instruments))
        .route("/api/v1/b3/instruments/{id}", get(release::instrument))
        .route(
            "/api/v1/b3/instruments/{id}/identifiers",
            get(release::instrument_identifiers),
        )
        .route(
            "/api/v1/b3/tradable-securities",
            get(release::tradable_securities),
        )
        .route("/api/v1/b3/cotahist/series", get(release::series))
        .route(
            "/api/v1/b3/jobs",
            get(release_ops::jobs).post(release::submit_job),
        )
        .route("/api/v1/b3/sync-runs", get(release_ops::sync_runs))
        .route("/api/v1/b3/capabilities", get(release_ops::capabilities))
        .route("/api/v1/b3/jobs/{id}", get(release::job))
        .route("/api/v1/b3/jobs/{id}/cancel", post(release::cancel_job))
        .route("/api/v1/b3/source-files", get(release_ops::source_files))
        .route(
            "/api/v1/b3/source-files/{id}/members",
            get(release_ops::members),
        )
        .route("/api/v1/b3/pending-updates", get(release_ops::pending))
        .route("/api/v1/b3/data-revisions/{id}", get(release_ops::revision))
        .route(
            "/api/v1/b3/data-revisions/{id}/conflicts",
            get(release_ops::conflicts),
        )
        .route(
            "/api/v1/b3/data-revisions/{id}/quarantine",
            get(release_ops::rejections),
        )
        .route("/api/v1/b3/discovery/scan", post(release_ops::scan))
        .route("/api/v1/b3/jobs/{id}/retry", post(release_extra::retry))
        .route(
            "/api/v1/b3/diagnostics/coverage",
            get(release_extra::diagnostics),
        )
        .route("/api/v1/b3/sync-runs/{id}", get(release_extra::sync_run))
        .route(
            "/api/v1/b3/pending-updates/{id}/acknowledge",
            post(release_ops::acknowledge_pending),
        )
        .fallback(|| async {
            release_error::ApiError::new(
                axum::http::StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Rota não publicada neste lançamento.",
            )
        })
        .layer(middleware::from_fn_with_state(
            state.settings.clone(),
            auth::require_bearer_token,
        ))
        .with_state(state)
}
