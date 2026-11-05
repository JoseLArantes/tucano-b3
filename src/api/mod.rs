pub mod auth;
pub mod routers;
pub mod schemas;

use axum::{
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use utoipa::OpenApi;

use crate::config::B3Settings;
use crate::services::sync_service::B3SyncService;

use self::auth::require_bearer_token;
use self::routers::{
    company_data::*, cotahist::*, equities_simplified::*, governance::*, health::*,
    index_composition::*, index_report::*, instruments::*, price_report::*, sync::*, tradable::*,
};

pub struct AppState {
    pub pool: PgPool,
    pub settings: Arc<B3Settings>,
    pub sync_service: Arc<B3SyncService>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tucano B3 Data Platform API",
        version = "0.1.0",
        description = "High-performance, cost-effective Rust API for B3 (Brasil, Bolsa, Balcão) exchange market data, historical series, and governance.\n\n### Authentication\nProtected endpoints require an `Authorization: Bearer <token>` header configured via `TUCANO_B3_TOKEN`.\n\n### Preços Históricos (COTAHIST)\nAVISO: As cotações da série histórica COTAHIST representam preços brutos nominais de negociação diária, sem ajustes retroativos por dividendos, JCP, desdobramentos ou bonificações corporativas."
    ),
    paths(
        routers::health::health_check,
        routers::instruments::list_instruments,
        routers::instruments::get_instrument,
        routers::tradable::list_tradable_securities,
        routers::price_report::list_price_report_quotes,
        routers::equities_simplified::list_equities_simplified_quotes,
        routers::index_report::list_indices,
        routers::index_report::list_index_values,
        routers::index_composition::list_index_compositions,
        routers::cotahist::list_cotahist_quotes,
        routers::sync::list_source_files,
        routers::sync::list_sync_runs,
        routers::sync::trigger_sync_run,
        routers::sync::rebuild_identity,
        routers::governance::list_quarantine_records,
        routers::governance::replay_quarantine,
        routers::governance::list_field_changes,
        routers::governance::list_pending_updates,
        routers::governance::get_coverage_diagnostics,
        routers::company_data::list_company_data,
    ),
    components(
        schemas(
            schemas::HealthResponse,
            schemas::Provenance,
            schemas::InternalMetadata,
            schemas::ManualSyncRequest,
            schemas::ManualSyncResponse,
            schemas::RebuildIdentityResponse,
            schemas::QuarantineReplayResponse,
            schemas::CoverageMetric,
            routers::instruments::InstrumentItem,
            routers::instruments::IdentifierItem,
            routers::tradable::TradableSecurityItem,
            routers::price_report::PriceReportQuoteItem,
            routers::equities_simplified::EquitiesSimplifiedQuoteItem,
            routers::equities_simplified::PriceComparisonItem,
            routers::index_report::IndexItem,
            routers::index_report::IndexValueItem,
            routers::index_composition::IndexCompositionItem,
            routers::cotahist::CotahistQuoteItem,
            routers::sync::SourceFileItem,
            routers::sync::SourceFileMemberItem,
            routers::sync::SyncRunItem,
            routers::governance::QuarantineItem,
            routers::governance::FieldChangeItem,
            routers::governance::PendingUpdateItem,
            routers::company_data::CompanyDataItem,
        )
    ),
    tags(
        (name = "Infrastructure & Health", description = "Health check and system diagnostics"),
        (name = "B3 Instruments File (BVBG.028.02)", description = "Official B3 security definitions and instruments"),
        (name = "B3 Tradable Security List", description = "Daily listing of securities eligible for trading"),
        (name = "B3 PriceReport (BVBG.086.01)", description = "Complete B3 end-of-day market price reports"),
        (name = "B3 Simplified Price Report - Equities (BVBG.186.01)", description = "Simplified equity trading prices"),
        (name = "B3 IndexReport (BVBG.087.01)", description = "B3 market indices (IBOV, etc.) and IOPV values"),
        (name = "B3 Index Portfolio Composition", description = "Constituent weights and theoretical quantities for B3 indices"),
        (name = "B3 COTAHIST Historical Quotes", description = "Nominal historical quotes (unadjusted raw prices)"),
        (name = "B3 Company Data", description = "Consolidated company profiles and identifiers"),
        (name = "B3 Source Files", description = "Provenance audit for downloaded B3 data files"),
        (name = "B3 Sync Jobs", description = "Manual and automated data ingestion jobs"),
        (name = "B3 Data Quality & Governance", description = "Quarantine, field change audits, and coverage diagnostics"),
    )
)]
pub struct ApiDoc;

pub async fn openapi_json() -> Response {
    let spec = ApiDoc::openapi();
    Json(spec).into_response()
}

pub async fn swagger_ui() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
  <title>Tucano B3 - Swagger UI</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = function() {
      SwaggerUIBundle({
        url: "/openapi.json",
        dom_id: '#swagger-ui',
        deepLinking: true,
        presets: [
          SwaggerUIBundle.presets.apis,
          SwaggerUIBundle.SwaggerUIStandalonePreset
        ]
      });
    };
  </script>
</body>
</html>"#)
}

pub async fn redoc_ui() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
  <title>Tucano B3 - ReDoc</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
  <style>
    body { margin: 0; padding: 0; }
  </style>
</head>
<body>
  <redoc spec-url="/openapi.json"></redoc>
  <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#)
}

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let settings = app_state.settings.clone();

    Router::new()
        // Unauthenticated docs & health
        .route("/healthz", get(healthz))
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/b3/worker/health", get(worker_health))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
        .route("/redoc", get(redoc_ui))

        // Protected B3 API routes
        // Instruments
        .route("/api/v1/b3/instruments", get(list_instruments))
        .route("/api/v1/b3/instruments/{id}", get(get_instrument))
        .route("/api/v1/b3/instruments/by-security-id/{security_id}", get(get_instrument))
        .route("/api/v1/b3/instruments/by-ticker/{ticker_symbol}", get(get_instrument_by_ticker))
        .route("/api/v1/b3/instruments/by-isin/{isin}", get(get_instrument_by_isin))
        .route("/api/v1/b3/instruments/{id}/identifiers", get(list_instrument_identifiers))

        // Tradable
        .route("/api/v1/b3/tradable-securities", get(list_tradable_securities))
        .route("/api/v1/b3/tradable-securities/by-date/{reference_date}", get(get_tradable_by_date))
        .route("/api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}", get(get_tradable_by_ticker))

        // Price Report (BVBG.086)
        .route("/api/v1/b3/price-report/quotes", get(list_price_report_quotes))
        .route("/api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}", get(get_price_report_by_ticker))
        .route("/api/v1/b3/price-report/quotes/by-date/{trade_date}", get(get_price_report_by_date))
        .route("/api/v1/b3/price-report/quotes/{quote_id}", get(get_price_report_by_id))

        // Simplified Equities (BVBG.186)
        .route("/api/v1/b3/equities-simplified-price-report/quotes", get(list_equities_simplified_quotes))
        .route("/api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}", get(get_equities_simplified_by_ticker))
        .route("/api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}", get(get_equities_simplified_by_date))
        .route("/api/v1/b3/equities-simplified-price-report/quotes/{quote_id}", get(get_equities_simplified_by_id))
        .route("/api/v1/b3/price-report-comparison/{trade_date}", get(compare_price_reports))

        // Index Report (BVBG.087)
        .route("/api/v1/b3/index-report/indices", get(list_indices))
        .route("/api/v1/b3/index-report/values", get(list_index_values))
        .route("/api/v1/b3/index-report/values/by-index/{index_code}", get(get_index_values_by_code))
        .route("/api/v1/b3/index-report/values/by-date/{trade_date}", get(get_index_values_by_date))

        // Index Portfolio Composition
        .route("/api/v1/b3/index-composition/portfolios", get(list_index_compositions))
        .route("/api/v1/b3/index-composition/portfolios/{index_code}", get(get_index_composition_by_index))
        .route("/api/v1/b3/index-composition/portfolios/{index_code}/by-date/{composition_date}", get(get_index_composition_by_index_and_date))
        .route("/api/v1/b3/index-composition/components/by-ticker/{ticker_symbol}", get(get_index_composition_by_ticker))

        // COTAHIST
        .route("/api/v1/b3/cotahist/quotes", get(list_cotahist_quotes))
        .route("/api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}", get(get_cotahist_by_ticker))
        .route("/api/v1/b3/cotahist/quotes/by-isin/{isin}", get(get_cotahist_by_isin))
        .route("/api/v1/b3/cotahist/quotes/by-date/{trade_date}", get(get_cotahist_by_date))
        .route("/api/v1/b3/cotahist/quotes/{quote_id}", get(get_cotahist_by_id))

        // Company Data
        .route("/api/v1/b3/company-data", get(list_company_data))

        // Source Files & Sync Jobs
        .route("/api/v1/b3/source-files", get(list_source_files))
        .route("/api/v1/b3/source-files/{id}", get(get_source_file_by_id))
        .route("/api/v1/b3/source-files/{id}/members", get(list_source_file_members))
        .route("/api/v1/b3/sync-runs", get(list_sync_runs).post(trigger_sync_run))
        .route("/api/v1/b3/sync-runs/{id}", get(get_sync_run_by_id))
        .route("/api/v1/b3/sync-runs/rebuild-identity", post(rebuild_identity))

        // Governance
        .route("/api/v1/b3/quarantine", get(list_quarantine_records))
        .route("/api/v1/b3/quarantine/replay", post(replay_quarantine))
        .route("/api/v1/b3/field-changes", get(list_field_changes))
        .route("/api/v1/b3/pending-updates", get(list_pending_updates))
        .route("/api/v1/b3/diagnostics/coverage", get(get_coverage_diagnostics))

        // Apply auth middleware to all protected routes
        .layer(middleware::from_fn_with_state(settings, require_bearer_token))
        .with_state(app_state)
}

