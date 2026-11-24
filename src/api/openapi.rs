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
use crate::models::market_calendar::*;
use crate::models::pagination::*;
use crate::models::proventos::*;

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
### Índices B3\n\
`/metrics/ibov/history` e `/metrics/ibov/latest` consultam a série oficial do IBOV. Também estão disponíveis `/metrics/idiv/history` e `/metrics/ifix/latest`. Os CSVs publicados pela B3 fornecem fechamento diário em pontos.\n\n\
### Calendário e negociação B3\n\
`/calendar` lista e cria calendários anuais; `/calendar/{year}` consulta, substitui ou remove o calendário do ano. O ledger inicia com a agenda oficial de 2026 e preserva as fontes. `/negotiation-hours` apresenta os horários gerais publicados para Tela, RFQ e Voice; exceções de uma data estão no calendário anual.\n\n\
### Proventos cadastrados\n\
CRUD autenticado de dividendos, juros sobre capital próprio (`JCP`) e rendimentos em `/proventos`. Os valores são decimais textuais exatos em reais; datas, fonte e observações ficam associadas a cada registro. A API preserva a proveniência informada, mas não valida as fontes nem apresenta proventos como série oficial da B3.\n\n\
### Ingestão e operações\n\
Upload, pasta local ou bucket S3 compatível (Cloudflare R2) de arquivos COTAHIST (`.ZIP`/`.TXT`) com idempotência por data de pregão. Os CSVs de IBOV, IDIV e IFIX são atualizados diariamente às 02:00 em `America/Sao_Paulo` e podem ser disparados por `POST /ingest/indices`. Health, info do dataset e métricas Prometheus.\n\n\
### Contratos e limitações\n\
- Preços COTAHIST são **nominais** (sem ajuste retroativo por eventos corporativos).\n\
- Autenticação Bearer / API Key nas rotas protegidas; `/health`, `/docs`, `/openapi.json` e o alias legado `/api/docs` são públicos.\n\
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
        crate::api::cotahist::list_equities_handler,
        crate::api::cotahist::get_equity_handler,
        crate::api::cotahist::latest_equity_quote_handler,
        crate::api::cotahist::equity_quotes_handler,
        crate::api::cotahist::equity_turnover_handler,
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
        // Official index metrics endpoints and manual ingestion
        crate::api::index_metrics::ibov_history,
        crate::api::index_metrics::ibov_latest,
        crate::api::index_metrics::idiv_history,
        crate::api::index_metrics::ifix_latest,
        crate::api::index_metrics::start_ingest,
        crate::api::index_metrics::ingest_status,
        crate::api::index_metrics::ingestion_overview,
        // B3 market calendar and Trader negotiation hours
        crate::api::market_calendar::list_calendars,
        crate::api::market_calendar::get_calendar,
        crate::api::market_calendar::create_calendar,
        crate::api::market_calendar::update_calendar,
        crate::api::market_calendar::delete_calendar,
        crate::api::market_calendar::negotiation_hours,
        // Source-attributed provento CRUD and reporting
        crate::api::proventos::list_proventos,
        crate::api::proventos::get_provento,
        crate::api::proventos::create_provento,
        crate::api::proventos::update_provento,
        crate::api::proventos::delete_provento,
        crate::api::proventos::reports::yearly,
        crate::api::proventos::reports::period,
        crate::api::proventos::reports::series,
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
            crate::ingest::index_metrics::IndexCode,
            crate::ingest::index_metrics::ManualIngestRequest,
            crate::ingest::index_metrics::IngestJob,
            crate::api::index_metrics::IndexIngestStatus,
            crate::api::index_metrics::IngestJobResponse,
            crate::api::index_metrics::IngestJobReceipt,
            // Envelope aliases
            TickerListApiResponse,
            EquityListApiResponse,
            EquityApiResponse,
            EquityQuoteApiResponse,
            EquityQuotesApiResponse,
            EquityTurnoverApiResponse,
            EquityTurnoverResponse,
            EquityTurnoverObservation,
            TurnoverSegment,
            EquityResponse,
            EquityQuoteResponse,
            EquityCoverage,
            EquityListResponse,
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
            MarketCalendarYearsApiResponse,
            MarketCalendarApiResponse,
            MarketCalendarDeleteApiResponse,
            NegotiationHoursApiResponse,
            ProventoApiResponse,
            ProventosListApiResponse,
            ProventoDeleteApiResponse,
            ProventosYearlyApiResponse,
            ProventosPeriodApiResponse,
            ProventosSeriesApiResponse,
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
            MarketDayStatus,
            MarketCalendarEvent,
            MarketCalendarCreateRequest,
            MarketCalendarUpdateRequest,
            MarketCalendarResponse,
            MarketCalendarYearsResponse,
            MarketCalendarDeleteResponse,
            NegotiationSessionHours,
            NegotiationHoursResponse,
            ProventoTipo,
            ProventoRequest,
            ProventoResponse,
            ProventosListResponse,
            ProventoDeleteResponse,
            ProventosYearlyResponse,
            ProventosPeriodTotal,
            ProventosSeriesPoint,
            ProventosTickerSeries,
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
        (name = "índices", description = "Séries oficiais de índices da B3"),
        (name = "calendário", description = "Calendário anual e horários de negociação publicados pela B3"),
        (name = "proventos", description = "CRUD de dividendos, JCP e rendimentos com fonte preservada e relatórios por data-com"),
        (name = "ingestão", description = "Ingestão de dados COTAHIST e índices da B3"),
        (name = "sistema", description = "Endpoints de saúde e informações do serviço"),
    ),
)]
pub struct ApiDoc;
