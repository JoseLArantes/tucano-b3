//! OpenAPI specification builder.
//!
//! Generates the OpenAPI 3.0 document from code annotations.
//! All descriptions are in Brazilian Portuguese.

use utoipa::{Modify, OpenApi};

use crate::error::ErrorResponse;
use crate::models::cotahist::*;
use crate::models::cotahist_analytics::*;
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
            components.add_security_scheme(
                "BearerAuth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::Http::new(
                        utoipa::openapi::security::HttpAuthScheme::Bearer,
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
        title = "API dados de mercado da B3",
        version = "0.1.0",
        description = "API REST de alta performance para dados oficiais de mercado da B3 (Brasil, Bolsa, Balcão).\n\n\
## Capabilidades\n\n\
### COTAHIST — cotações históricas\n\
- Descoberta de instrumentos (`/cotahist/tickers`, detalhe por ticker).\n\
- Histórico OHLCV paginado, cotação mais recente e cross-section por data de pregão.\n\
- Série compacta para gráficos (`/series`: `date_ms`, `pre_ult`, `vol_tot`, `tot_neg`) e extremos de fechamento (`/pre-ult/min`, `/pre-ult/max`).\n\n\
### Analytics derivadas (preços nominais)\n\
Indicadores calculados sobre PREULT/OHLCV COTAHIST — **não** são séries oficiais publicadas pela B3. Toda resposta analítica inclui o campo `aviso`.\n\
- Retorno e anualização (`/returns`), cartão estatístico (`/stats`), drawdown (`/drawdown`), posição no intervalo (`/range-position`).\n\
- Performance relativa vs IBOV (`/relative`), movers do pregão (`/dates/{date}/movers`), tela de liquidez (`/liquidity`).\n\
- Gaps de abertura (`/gaps`), indicadores móveis SMA/EMA/vol/ADV (`/rolling`), spread compra/venda (`/spread`), comparação normalizada base 100 (`/compare`).\n\n\
### IBOV\n\
Série histórica e ponto mais recente do índice IBOVESPA.\n\n\
### Ingestão e operações\n\
Upload, pasta local ou bucket S3 compatível (Cloudflare R2) de arquivos COTAHIST (`.ZIP`/`.TXT`) com idempotência por data de pregão. A ingestão do bucket só ocorre em `POST /ingest/bucket`. Health, info do dataset e métricas Prometheus.\n\n\
### Contratos e limitações\n\
- Preços COTAHIST são **nominais** (sem ajuste retroativo por eventos corporativos).\n\
- Autenticação Bearer / API Key nas rotas protegidas; `/health` e `/api/docs` são públicos.\n\
- Envelope padrão `{ data, meta }` em respostas de sucesso; erros tipados em português.",
        contact(
            name = "tucano-b3",
            url = "https://github.com/tucano-b3/tucano-b3",
        ),
        license(
            name = "MIT",
        ),
    ),
    servers(
        (url = "/api/v1", description = "Servidor principal"),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
    paths(
        // COTAHIST endpoints
        crate::api::cotahist::list_tickers_handler,
        crate::api::cotahist::get_ticker_handler,
        crate::api::cotahist::get_quotes_handler,
        crate::api::cotahist::get_latest_handler,
        crate::api::cotahist::get_series_handler,
        crate::api::cotahist::get_pre_ult_min_handler,
        crate::api::cotahist::get_pre_ult_max_handler,
        crate::api::cotahist::get_date_quotes_handler,
        // COTAHIST analytics
        crate::api::cotahist_analytics::get_returns_handler,
        crate::api::cotahist_analytics::get_stats_handler,
        crate::api::cotahist_analytics::get_drawdown_handler,
        crate::api::cotahist_analytics::get_range_position_handler,
        crate::api::cotahist_analytics::get_relative_handler,
        crate::api::cotahist_analytics::get_movers_handler,
        crate::api::cotahist_analytics::get_liquidity_handler,
        crate::api::cotahist_analytics::get_gaps_handler,
        crate::api::cotahist_analytics::get_rolling_handler,
        crate::api::cotahist_analytics::get_spread_handler,
        crate::api::cotahist_analytics::get_compare_handler,
        // IBOV endpoints
        crate::api::ibov::history_handler,
        crate::api::ibov::latest_handler,
        // System endpoints
        crate::api::health::health_handler,
        crate::api::info::info_handler,
        // Ingestion endpoints
        crate::api::ingest::upload_handler,
        crate::api::ingest::folder_handler,
        crate::api::ingest::bucket_handler,
    ),
    components(
        schemas(
            // Envelope aliases
            TickerListApiResponse,
            TickerDetailApiResponse,
            QuoteListApiResponse,
            LatestQuoteApiResponse,
            DateQuotesApiResponse,
            SeriesApiResponse,
            PreUltExtremeApiResponse,
            ReturnsApiResponse,
            StatsApiResponse,
            DrawdownApiResponse,
            RangePositionApiResponse,
            RelativeApiResponse,
            MoversApiResponse,
            LiquidityScreenApiResponse,
            GapsApiResponse,
            RollingApiResponse,
            SpreadApiResponse,
            CompareApiResponse,
            IbovHistoryApiResponse,
            IbovLatestApiResponse,
            IngestApiResponse,
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
            SeriesPoint,
            SeriesResponse,
            PreUltExtremeResponse,
            // COTAHIST analytics
            ReturnsResponse,
            StatsResponse,
            DrawdownResponse,
            RangePositionResponse,
            RelativeResponse,
            MoverItem,
            MoversResponse,
            LiquidityItem,
            LiquidityScreenResponse,
            GapItem,
            GapsResponse,
            RollingPoint,
            RollingResponse,
            SpreadPoint,
            SpreadResponse,
            CompareSeries,
            CompareResponse,
            // IBOV
            IbovHistoryEntry,
            IbovHistoryResponse,
            IbovLatestResponse,
            // Ingest
            crate::models::ingest::IngestResponse,
            crate::models::ingest::FolderIngestRequest,
            crate::models::ingest::BucketIngestRequest,
            crate::models::ingest::UploadFileRequest,
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
