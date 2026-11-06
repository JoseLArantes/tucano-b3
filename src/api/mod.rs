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
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

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

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Token")
                        .description(Some("Token de autenticação Tucano B3 (ex: dev-token)"))
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    security(
        ("bearer_auth" = [])
    ),
    info(
        title = "API Tucano B3 - Plataforma de Dados de Mercado",
        version = "0.1.0",
        description = "
API de alta performance para ingestão, normalização, auditoria temporal e exposição pública de dados oficiais da **B3 (Brasil, Bolsa, Balcão)**.

### Princípios Arquiteturais e Pilares de Engenharia:

1. **Escopo Estrito B3-Only:**
   - Todo dado é rastreado a uma fonte oficial primária da B3 (`BVBG.028.02`, `BVBG.086.01`, `BVBG.186.01`, `BVBG.087.01`, `Tradable Security List`, Carteiras de Índices e `COTAHIST`).
   - Nenhuma fonte externa (CVM, Yahoo Finance, portais ou corretoras) é utilizada. Identificadores como CNPJ de emissor ou código CVM não são âncoras cadastrais neste serviço.

2. **Idempotência Absoluta e Auditoria Granular:**
   - O reprocessamento do mesmo arquivo com o mesmo conteúdo não gera duplicatas, erros de unicidade ou escritas redundantes.
   - Detecção de mudanças em nível de atributo (`IS DISTINCT FROM`): apenas campos de negócio efetivamente modificados são atualizados, gerando histórico temporal imutável em `b3_field_changes` e timestamps isolados em `field_changed_at`.
   - Chaves compostas com campos nulos utilizam `UNIQUE NULLS NOT DISTINCT` nativo do PostgreSQL 16.

3. **Coordenação Distribuída sem Redis/Celery (Zero Redis):**
   - O controle de concorrência global e por data de referência é garantido exclusivamente por travas consultivas ACID do PostgreSQL 16 (`pg_try_advisory_lock` e `pg_try_advisory_xact_lock`).
   - Tentativas de ingestão simultânea para o mesmo arquivo/data retornam imediatamente HTTP `409 Conflict`.

4. **Resolução Temporal de Identidades (Relationship Resolver):**
   - A raiz cadastral primária é o `security_id` da estrutura de negociação da B3 (`BVBG.028.02`).
   - Identificadores voláteis (tickers, ISINs) são gerenciados em janelas temporais de validade (`valid_from` até `valid_to`), permitindo reuso histórico de tickers sem colisões.
   - Colisões ambíguas não são atribuídas arbitrariamente: registros permanecem como `ambiguous` com `instrument_id = NULL` até resolução unívoca.

5. **Isolamento Estrito entre Famílias de Arquivos:**
   - `BVBG.086.01` (PriceReport abrangente) e `BVBG.186.01` (Simplified Equities) são armazenados em tabelas e endpoints distintos, sem mesclagem silenciosa.
   - O endpoint `/api/v1/b3/price-report-comparison/{trade_date}` permite auditoria cruzada de consistência entre ambos.

### AVISO REGULATÓRIO OBRIGATÓRIO: DADOS DE PREÇO COTAHIST

> **AVISO IMPORTANTE:** As cotações da série histórica COTAHIST representam **preços brutos nominais (unadjusted)** praticados na data do pregão.
> Eles **NÃO sofrem ajuste retroativo** por eventos corporativos em dinheiro (dividendos, JCP) ou em ações (desdobramentos/splits, grupamentos e bonificações).
> Séries ajustadas requerem cruzamento ex-dividendo externo com a base de eventos corporativos (ex: CVM).

### Formatação Numérica e Precisão:
- Todos os valores financeiros, quantidades e pontuações utilizam aritmética exata de ponto fixo (`rust_decimal::Decimal`), sem conversão por float.
- Na API REST, decimais são serializados como strings canônicas sem separador de milhar (ex.: `\"36.00\"`).

### Autenticação:
- Todos os endpoints (exceto `/healthz`, `/metrics`, `/api/v1/health`, `/openapi.json`, `/docs` e `/redoc`) exigem cabeçalho HTTP:
  `Authorization: Bearer <TUCANO_B3_TOKEN>`.
"
    ),
    paths(
        routers::health::health_check,
        routers::health::worker_health,
        routers::health::metrics,
        routers::instruments::list_instruments,
        routers::instruments::get_instrument,
        routers::instruments::get_instrument_by_ticker,
        routers::instruments::get_instrument_by_isin,
        routers::instruments::list_instrument_identifiers,
        routers::tradable::list_tradable_securities,
        routers::tradable::get_tradable_by_date,
        routers::tradable::get_tradable_by_ticker,
        routers::price_report::list_price_report_quotes,
        routers::price_report::get_price_report_by_ticker,
        routers::price_report::get_price_report_by_date,
        routers::price_report::get_price_report_by_id,
        routers::equities_simplified::list_equities_simplified_quotes,
        routers::equities_simplified::get_equities_simplified_by_ticker,
        routers::equities_simplified::get_equities_simplified_by_date,
        routers::equities_simplified::get_equities_simplified_by_id,
        routers::equities_simplified::compare_price_reports,
        routers::index_report::list_indices,
        routers::index_report::list_index_values,
        routers::index_report::get_index_values_by_code,
        routers::index_report::get_index_values_by_date,
        routers::index_composition::list_index_compositions,
        routers::index_composition::get_index_composition_by_index,
        routers::index_composition::get_index_composition_by_index_and_date,
        routers::index_composition::get_index_composition_by_ticker,
        routers::cotahist::list_cotahist_quotes,
        routers::cotahist::get_cotahist_by_ticker,
        routers::cotahist::get_cotahist_by_isin,
        routers::cotahist::get_cotahist_by_date,
        routers::cotahist::get_cotahist_by_id,
        routers::company_data::list_company_data,
        routers::sync::list_source_files,
        routers::sync::get_source_file_by_id,
        routers::sync::list_source_file_members,
        routers::sync::list_sync_runs,
        routers::sync::get_sync_run_by_id,
        routers::sync::trigger_sync_run,
        routers::sync::rebuild_identity,
        routers::governance::list_quarantine_records,
        routers::governance::replay_quarantine,
        routers::governance::list_field_changes,
        routers::governance::list_pending_updates,
        routers::governance::get_coverage_diagnostics,
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
        (name = "Infrastructure & Health", description = "Verificação de disponibilidade da API, worker daemon e exportação de métricas Prometheus."),
        (name = "B3 Instruments File (BVBG.028.02)", description = "Catálogo oficial de instrumentos financeiros e histórico de identificadores temporais."),
        (name = "B3 Tradable Security List", description = "Relação diária de títulos autorizados para negociação em sessão de mercado."),
        (name = "B3 PriceReport (BVBG.086.01)", description = "Boletim diário completo de preços de mercado, negociações e volumes."),
        (name = "B3 Simplified Price Report - Equities (BVBG.186.01)", description = "Boletim simplificado de ações à vista e comparativo cruzado contra o PriceReport."),
        (name = "B3 IndexReport (BVBG.087.01)", description = "Valores de fechamento, médias e pontuações dos índices oficiais calculados pela B3."),
        (name = "B3 Index Portfolio Composition", description = "Carteiras teóricas oficiais, quantidades de ações e participações percentuais dos ativos em índices."),
        (name = "B3 COTAHIST Historical Quotes", description = "Séries históricas nominais brutas (unadjusted) com aviso regulatório obrigatório."),
        (name = "B3 Company Data", description = "Visão consolidada de companhias e emissores derivada estritamente da base de instrumentos B3."),
        (name = "B3 Source Files", description = "Auditoria de arquivos físicos baixados da B3, checksums SHA-256 e membros de containers ZIP."),
        (name = "B3 Sync Jobs", description = "Disparo sob demanda de ingestão com travas consultivas e reconstrução de identidades."),
        (name = "B3 Data Quality & Governance", description = "Governança de dados: quarentena, replay, histórico de campos alterados e métricas de cobertura."),
    )
)]
pub struct ApiDoc;

pub async fn openapi_json() -> Response {
    let spec = ApiDoc::openapi();
    Json(spec).into_response()
}

pub async fn swagger_ui() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html>
<head>
  <title>Tucano B3 - Swagger UI</title>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
  <link rel="icon" type="image/png" href="https://unpkg.com/swagger-ui-dist@5/favicon-32x32.png" sizes="32x32" />
  <style>
    html { box-sizing: border-box; overflow: -moz-scrollbars-vertical; overflow-y: scroll; }
    *, *:before, *:after { box-sizing: inherit; }
    body { margin: 0; background: #fafafa; }
    .topbar { display: none; }
  </style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
  <script>
    window.onload = function() {
      window.ui = SwaggerUIBundle({
        url: "/openapi.json",
        dom_id: '#swagger-ui',
        deepLinking: true,
        persistAuthorization: true,
        displayRequestDuration: true,
        docExpansion: 'list',
        filter: true,
        presets: [
          SwaggerUIBundle.presets.apis,
          SwaggerUIStandalonePreset
        ],
        layout: "BaseLayout"
      });
    };
  </script>
</body>
</html>"#,
    )
}

pub async fn redoc_ui() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
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
  <redoc spec-url="/openapi.json" expand-responses="200,201"></redoc>
  <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#,
    )
}

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let settings = app_state.settings.clone();

    Router::new()
        // Unauthenticated docs & health
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics))
        .route("/api/v1/health", get(health_check))
        .route("/api/v1/b3/worker/health", get(worker_health))
        .route("/openapi.json", get(openapi_json))
        .route("/docs", get(swagger_ui))
        .route("/docs/", get(swagger_ui))
        .route("/swagger-ui", get(swagger_ui))
        .route("/swagger-ui/", get(swagger_ui))
        .route("/redoc", get(redoc_ui))
        .route("/redoc/", get(redoc_ui))
        // Protected B3 API routes
        // Instruments
        .route("/api/v1/b3/instruments", get(list_instruments))
        .route("/api/v1/b3/instruments/{id}", get(get_instrument))
        .route(
            "/api/v1/b3/instruments/by-security-id/{security_id}",
            get(get_instrument),
        )
        .route(
            "/api/v1/b3/instruments/by-ticker/{ticker_symbol}",
            get(get_instrument_by_ticker),
        )
        .route(
            "/api/v1/b3/instruments/by-isin/{isin}",
            get(get_instrument_by_isin),
        )
        .route(
            "/api/v1/b3/instruments/{id}/identifiers",
            get(list_instrument_identifiers),
        )
        // Tradable
        .route(
            "/api/v1/b3/tradable-securities",
            get(list_tradable_securities),
        )
        .route(
            "/api/v1/b3/tradable-securities/by-date/{reference_date}",
            get(get_tradable_by_date),
        )
        .route(
            "/api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}",
            get(get_tradable_by_ticker),
        )
        // Price Report (BVBG.086)
        .route(
            "/api/v1/b3/price-report/quotes",
            get(list_price_report_quotes),
        )
        .route(
            "/api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}",
            get(get_price_report_by_ticker),
        )
        .route(
            "/api/v1/b3/price-report/quotes/by-date/{trade_date}",
            get(get_price_report_by_date),
        )
        .route(
            "/api/v1/b3/price-report/quotes/{quote_id}",
            get(get_price_report_by_id),
        )
        // Simplified Equities (BVBG.186)
        .route(
            "/api/v1/b3/equities-simplified-price-report/quotes",
            get(list_equities_simplified_quotes),
        )
        .route(
            "/api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}",
            get(get_equities_simplified_by_ticker),
        )
        .route(
            "/api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}",
            get(get_equities_simplified_by_date),
        )
        .route(
            "/api/v1/b3/equities-simplified-price-report/quotes/{quote_id}",
            get(get_equities_simplified_by_id),
        )
        .route(
            "/api/v1/b3/price-report-comparison/{trade_date}",
            get(compare_price_reports),
        )
        // Index Report (BVBG.087)
        .route("/api/v1/b3/index-report/indices", get(list_indices))
        .route("/api/v1/b3/index-report/values", get(list_index_values))
        .route(
            "/api/v1/b3/index-report/values/by-index/{index_code}",
            get(get_index_values_by_code),
        )
        .route(
            "/api/v1/b3/index-report/values/by-date/{trade_date}",
            get(get_index_values_by_date),
        )
        // Index Portfolio Composition
        .route(
            "/api/v1/b3/index-composition/portfolios",
            get(list_index_compositions),
        )
        .route(
            "/api/v1/b3/index-composition/portfolios/{index_code}",
            get(get_index_composition_by_index),
        )
        .route(
            "/api/v1/b3/index-composition/portfolios/{index_code}/by-date/{composition_date}",
            get(get_index_composition_by_index_and_date),
        )
        .route(
            "/api/v1/b3/index-composition/components/by-ticker/{ticker_symbol}",
            get(get_index_composition_by_ticker),
        )
        // COTAHIST
        .route("/api/v1/b3/cotahist/quotes", get(list_cotahist_quotes))
        .route(
            "/api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}",
            get(get_cotahist_by_ticker),
        )
        .route(
            "/api/v1/b3/cotahist/quotes/by-isin/{isin}",
            get(get_cotahist_by_isin),
        )
        .route(
            "/api/v1/b3/cotahist/quotes/by-date/{trade_date}",
            get(get_cotahist_by_date),
        )
        .route(
            "/api/v1/b3/cotahist/quotes/{quote_id}",
            get(get_cotahist_by_id),
        )
        // Company Data
        .route("/api/v1/b3/company-data", get(list_company_data))
        // Source Files & Sync Jobs
        .route("/api/v1/b3/source-files", get(list_source_files))
        .route("/api/v1/b3/source-files/{id}", get(get_source_file_by_id))
        .route(
            "/api/v1/b3/source-files/{id}/members",
            get(list_source_file_members),
        )
        .route(
            "/api/v1/b3/sync-runs",
            get(list_sync_runs).post(trigger_sync_run),
        )
        .route("/api/v1/b3/sync-runs/{id}", get(get_sync_run_by_id))
        .route(
            "/api/v1/b3/sync-runs/rebuild-identity",
            post(rebuild_identity),
        )
        // Governance
        .route("/api/v1/b3/quarantine", get(list_quarantine_records))
        .route("/api/v1/b3/quarantine/replay", post(replay_quarantine))
        .route("/api/v1/b3/field-changes", get(list_field_changes))
        .route("/api/v1/b3/pending-updates", get(list_pending_updates))
        .route(
            "/api/v1/b3/diagnostics/coverage",
            get(get_coverage_diagnostics),
        )
        // Apply auth middleware to all protected routes
        .layer(middleware::from_fn_with_state(
            settings,
            require_bearer_token,
        ))
        .with_state(app_state)
}
