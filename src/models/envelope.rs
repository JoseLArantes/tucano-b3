//! API response envelope.
//!
//! Every successful response is wrapped in ApiResponse<T>.
//! Every error response is wrapped in ApiError (defined in src/error.rs).

use serde::Serialize;
use utoipa::ToSchema;

use crate::models::cotahist::{
    DateQuotesResponse, LatestQuoteResponse, PreUltExtremeResponse, QuoteListResponse,
    SeriesResponse, TickerDetailResponse, TickerListResponse,
};
use crate::models::cotahist_analytics::{
    CompareResponse, DrawdownResponse, GapsResponse, LiquidityScreenResponse, MoversResponse,
    RangePositionResponse, RelativeResponse, ReturnsResponse, RollingResponse, SpreadResponse,
    StatsResponse,
};
use crate::models::ibov::{IbovHistoryResponse, IbovLatestResponse};
use crate::models::ingest::IngestResponse;

/// Standard response envelope for all successful API responses.
#[derive(Debug, Serialize, ToSchema)]
#[aliases(
    TickerListApiResponse = ApiResponse<TickerListResponse>,
    TickerDetailApiResponse = ApiResponse<TickerDetailResponse>,
    QuoteListApiResponse = ApiResponse<QuoteListResponse>,
    LatestQuoteApiResponse = ApiResponse<LatestQuoteResponse>,
    DateQuotesApiResponse = ApiResponse<DateQuotesResponse>,
    SeriesApiResponse = ApiResponse<SeriesResponse>,
    PreUltExtremeApiResponse = ApiResponse<PreUltExtremeResponse>,
    ReturnsApiResponse = ApiResponse<ReturnsResponse>,
    StatsApiResponse = ApiResponse<StatsResponse>,
    DrawdownApiResponse = ApiResponse<DrawdownResponse>,
    RangePositionApiResponse = ApiResponse<RangePositionResponse>,
    RelativeApiResponse = ApiResponse<RelativeResponse>,
    MoversApiResponse = ApiResponse<MoversResponse>,
    LiquidityScreenApiResponse = ApiResponse<LiquidityScreenResponse>,
    GapsApiResponse = ApiResponse<GapsResponse>,
    RollingApiResponse = ApiResponse<RollingResponse>,
    SpreadApiResponse = ApiResponse<SpreadResponse>,
    CompareApiResponse = ApiResponse<CompareResponse>,
    IbovHistoryApiResponse = ApiResponse<IbovHistoryResponse>,
    IbovLatestApiResponse = ApiResponse<IbovLatestResponse>,
    IngestApiResponse = ApiResponse<IngestResponse>,
)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    pub meta: ResponseMeta,
}

/// Metadata included in every successful response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ResponseMeta {
    /// Unique request identifier (UUID v4)
    pub request_id: String,
    /// ISO 8601 timestamp of the response
    pub timestamp: String,
    /// Whether the response was served from cache
    pub cached: bool,
    /// Query execution time in milliseconds
    pub query_time_ms: f64,
}

impl ResponseMeta {
    /// Create a new ResponseMeta for a non-cached response.
    ///
    /// Uses pre-formatted timestamp to minimize allocation.
    pub fn new(query_time_ms: f64) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            cached: false,
            query_time_ms,
        }
    }

    /// Create a new ResponseMeta for a cached response.
    pub fn cached(query_time_ms: f64) -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            cached: true,
            query_time_ms,
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    /// Wrap data in a standard response envelope.
    pub fn new(data: T, query_time_ms: f64) -> Self {
        Self {
            data,
            meta: ResponseMeta::new(query_time_ms),
        }
    }
}
