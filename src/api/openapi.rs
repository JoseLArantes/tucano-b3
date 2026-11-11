//! OpenAPI specification builder.
//!
//! Generates the OpenAPI 3.0 document from code annotations.
//! All descriptions are in Brazilian Portuguese.

use utoipa::{Modify, OpenApi};

use crate::error::ErrorResponse;
use crate::models::cotahist::*;
use crate::models::envelope::*;
use crate::models::ibov::*;
use crate::models::pagination::*;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "ApiKeyAuth",
                utoipa::openapi::security::SecurityScheme::ApiKey(
                    utoipa::openapi::security::ApiKey::Header(
                        utoipa::openapi::security::ApiKeyValue::new("X-API-Key"),
                    ),
                ),
            );
        }
    }
}

/// OpenAPI document for tucano-b3.
///
/// All endpoint handlers must be listed in the `paths` attribute.
/// All schemas must be listed in the `components` attribute.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "tucano-b3",
        version = "0.1.0",
        description = "API REST de alta performance para dados de mercado da B3 (Brasil, Bolsa, Balcão).\n\nEsta API serve dados históricos de cotações (COTAHIST) e a série histórica do IBOVESPA, ingeridos via `tucano-b3-cli`.",
        contact(
            name = "tucano-b3",
            url = "https://github.com/tucano-b3/tucano-b3",
        ),
        license(
            name = "MIT OR Apache-2.0",
        ),
    ),
    servers(
        (url = "/api/v1", description = "Servidor principal"),
    ),
    paths(
        // COTAHIST endpoints
        crate::api::cotahist::list_tickers_handler,
        crate::api::cotahist::get_ticker_handler,
        crate::api::cotahist::get_quotes_handler,
        crate::api::cotahist::get_latest_handler,
        crate::api::cotahist::get_date_quotes_handler,
        // IBOV endpoints
        crate::api::ibov::history_handler,
        crate::api::ibov::latest_handler,
        // System endpoints
        crate::api::health::health_handler,
        crate::api::info::info_handler,
        // Ingestion endpoints
        crate::api::ingest::upload_handler,
        crate::api::ingest::folder_handler,
    ),
    components(
        schemas(
            // Envelope
            ApiResponse<String>,
            ResponseMeta,
            ErrorResponse,
            crate::error::ErrorDetail,
            // Pagination
            PaginationMeta,
            // COTAHIST
            QuoteResponse,
            TickerResponse,
            TickerDetailResponse,
            TickerListResponse,
            QuoteListResponse,
            LatestQuoteResponse,
            DateQuotesResponse,
            // IBOV
            IbovHistoryEntry,
            IbovHistoryResponse,
            IbovLatestResponse,
            // Ingest
            crate::api::ingest::IngestResponse,
            crate::api::ingest::FolderIngestRequest,
            // Health
            crate::api::health::HealthResponse,
            crate::api::info::InfoResponse,
        ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "cotahist", description = "Cotações históricas de ativos listados na B3"),
        (name = "ibov", description = "Série histórica do índice IBOVESPA"),
        (name = "ingest", description = "Ingestão de dados COTAHIST"),
        (name = "sistema", description = "Endpoints de saúde e informações do serviço"),
    ),
)]
pub struct ApiDoc;
