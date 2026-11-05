use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceFileType {
    Bvbg028InstrumentsFile,
    TradableSecurityList,
    Bvbg086PriceReport,
    Bvbg186EquitiesSimplifiedPriceReport,
    Bvbg087IndexReport,
    B3IndexPortfolioComposition,
    CotahistYearly,
    CotahistMonthly,
    CotahistDaily,
}

impl SourceFileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bvbg028InstrumentsFile => "BVBG_028_INSTRUMENTS_FILE",
            Self::TradableSecurityList => "TRADABLE_SECURITY_LIST",
            Self::Bvbg086PriceReport => "BVBG_086_PRICE_REPORT",
            Self::Bvbg186EquitiesSimplifiedPriceReport => {
                "BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT"
            }
            Self::Bvbg087IndexReport => "BVBG_087_INDEX_REPORT",
            Self::B3IndexPortfolioComposition => "B3_INDEX_PORTFOLIO_COMPOSITION",
            Self::CotahistYearly => "COTAHIST_YEARLY",
            Self::CotahistMonthly => "COTAHIST_MONTHLY",
            Self::CotahistDaily => "COTAHIST_DAILY",
        }
    }
}

impl fmt::Display for SourceFileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for SourceFileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "BVBG_028_INSTRUMENTS_FILE" | "BVBG_028" => Ok(Self::Bvbg028InstrumentsFile),
            "TRADABLE_SECURITY_LIST" => Ok(Self::TradableSecurityList),
            "BVBG_086_PRICE_REPORT" | "BVBG_086" => Ok(Self::Bvbg086PriceReport),
            "BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT" | "BVBG_186" => {
                Ok(Self::Bvbg186EquitiesSimplifiedPriceReport)
            }
            "BVBG_087_INDEX_REPORT" | "BVBG_087" => Ok(Self::Bvbg087IndexReport),
            "B3_INDEX_PORTFOLIO_COMPOSITION" => Ok(Self::B3IndexPortfolioComposition),
            "COTAHIST_YEARLY" => Ok(Self::CotahistYearly),
            "COTAHIST_MONTHLY" => Ok(Self::CotahistMonthly),
            "COTAHIST_DAILY" => Ok(Self::CotahistDaily),
            other => Err(format!("Unknown source file type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFileStatus {
    Downloaded,
    Parsed,
    Ingested,
    SkippedDuplicate,
    Failed,
}

impl SourceFileStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Downloaded => "downloaded",
            Self::Parsed => "parsed",
            Self::Ingested => "ingested",
            Self::SkippedDuplicate => "skipped_duplicate",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for SourceFileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncRunStatus {
    Pending,
    Running,
    Success,
    Failed,
}

impl SyncRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

impl fmt::Display for SyncRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Resolved,
    Unresolved,
    Ambiguous,
}

impl ResolutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Unresolved => "unresolved",
            Self::Ambiguous => "ambiguous",
        }
    }
}

impl fmt::Display for ResolutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentIdentifierType {
    Ticker,
    Isin,
    SecurityId,
    Asset,
    UnderlyingTicker,
}

impl InstrumentIdentifierType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ticker => "ticker",
            Self::Isin => "isin",
            Self::SecurityId => "security_id",
            Self::Asset => "asset",
            Self::UnderlyingTicker => "underlying_ticker",
        }
    }
}

impl fmt::Display for InstrumentIdentifierType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
