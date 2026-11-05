use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PaginationParams {
    #[param(default = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<i64>,
    #[param(default = 0, minimum = 0)]
    pub offset: Option<i64>,
    #[param(default = false)]
    pub include_internal: Option<bool>,
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(100).clamp(1, 1000)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }

    pub fn include_internal(&self) -> bool {
        self.include_internal.unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Provenance {
    pub source_file_type: String,
    pub source_file_id: String,
    pub source_file_name: Option<String>,
    pub source_member_name: Option<String>,
    pub reference_date: Option<NaiveDate>,
    pub sha256: Option<String>,
    pub ingested_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InternalMetadata {
    pub data_changed_at: Option<DateTime<Utc>>,
    pub field_changed_at: Option<serde_json::Value>,
    pub resolution_status: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub source_line_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disclaimer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub app: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub database: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManualSyncRequest {
    pub source_file_type: String,
    pub reference_date_from: NaiveDate,
    pub reference_date_to: NaiveDate,
    pub source_url: Option<String>,
    pub force_download: Option<bool>,
    pub force_reparse: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManualSyncResponse {
    pub sync_run_id: String,
    pub status: String,
    pub inserted: i32,
    pub changed: i32,
    pub unchanged: i32,
    pub unresolved: i32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RebuildIdentityResponse {
    pub resolved_count: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuarantineReplayResponse {
    pub reprocessed_count: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CoverageMetric {
    pub dataset: String,
    pub min_date: Option<NaiveDate>,
    pub max_date: Option<NaiveDate>,
    pub total_records: i64,
    pub total_files: i64,
}
