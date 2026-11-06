//! Fail-closed eligibility checks. Evidence must be reviewed outside the ingestion pipeline.
use sqlx::PgPool;

use crate::{
    db::enums::SourceFileType,
    error::{AppError, Result},
};

/// Eligibility belongs to a source/purpose pair, not a hostname.
#[derive(Clone, Copy)]
pub enum SourcePurpose {
    Acquisition,
    RawRetention,
    Processing,
    Rest,
    Mcp,
    ThirdPartyDelivery,
}

impl SourcePurpose {
    fn as_str(self) -> &'static str {
        match self {
            Self::Acquisition => "acquisition",
            Self::RawRetention => "raw_retention",
            Self::Processing => "processing",
            Self::Rest => "rest",
            Self::Mcp => "mcp",
            Self::ThirdPartyDelivery => "third_party_delivery",
        }
    }
}

/// Every requested purpose must have dated, zero-cost evidence. Missing rows deny use.
pub async fn require_eligible(
    pool: &PgPool,
    source: SourceFileType,
    channel: &str,
    purposes: &[SourcePurpose],
) -> Result<()> {
    if purposes.is_empty() || channel.trim().is_empty() || channel == "unspecified" {
        return Err(AppError::Validation(
            "Finalidade de uso não informada.".into(),
        ));
    }
    let purposes: Vec<_> = purposes.iter().map(|p| p.as_str()).collect();
    let allowed: bool = sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM unnest($2::text[]) AS requested(purpose) WHERE NOT EXISTS (SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=$1 AND e.purpose=requested.purpose AND e.channel=$3 AND e.status='eligible' AND e.zero_cost AND length(trim(e.evidence_reference))>0 AND e.verified_at IS NOT NULL))")
        .bind(source.as_str()).bind(purposes).bind(channel).fetch_one(pool).await?;
    if !allowed {
        return Err(AppError::Validation(
            "Fonte/finalidade sem evidência de elegibilidade gratuita.".into(),
        ));
    }
    Ok(())
}
