use super::{ListFilters, OperationPage, QueryError, QueryService};
use crate::db::enums::SourceFileType;
use serde::Serialize;
use sqlx::Row;
use utoipa::ToSchema;

/// Capacidade da família, distinta de cobertura histórica e de autorização do consumidor.
#[derive(Serialize, ToSchema)]
pub struct Capability {
    /// Família oficial preservada separadamente.
    pub source_file_type: SourceFileType,
    /// Se o código de interpretação/consulta integra 0.0.1.
    pub implemented: bool,
    /// Se existe canal configurado elegível para REST e terceiros; não prova publicação de fatos.
    pub enabled: bool,
    /// Estado explícito da capacidade.
    pub status: CapabilityStatus,
}
/// Estados fechados do catálogo público de capacidades.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Enabled,
    NotEnabled,
    Deferred,
}

impl QueryService {
    pub async fn capabilities(
        &self,
        filters: ListFilters,
    ) -> Result<OperationPage<Capability>, QueryError> {
        use SourceFileType::*;
        let (l, o) = filters.validate()?;
        let enabled=sqlx::query("SELECT DISTINCT c.source_file_type FROM b3_source_channels c WHERE c.enabled AND NOT EXISTS(SELECT 1 FROM unnest(ARRAY['rest','third_party_delivery']) p(purpose) WHERE NOT EXISTS(SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=c.source_file_type AND e.channel=c.id AND e.purpose=p.purpose AND e.status='eligible' AND e.zero_cost AND length(trim(e.evidence_reference))>0 AND e.verified_at IS NOT NULL))").fetch_all(&self.pool).await?;
        let enabled: Vec<String> = enabled
            .into_iter()
            .map(|r| r.get("source_file_type"))
            .collect();
        let items = [
            Bvbg028InstrumentsFile,
            TradableSecurityList,
            CotahistYearly,
            CotahistMonthly,
            CotahistDaily,
            Bvbg086PriceReport,
            Bvbg186EquitiesSimplifiedPriceReport,
            Bvbg087IndexReport,
            B3IndexPortfolioComposition,
        ]
        .into_iter()
        .enumerate()
        .skip(o as usize)
        .take(l as usize)
        .map(|(i, source)| {
            let implemented = i < 5;
            let available = implemented && enabled.iter().any(|s| s == source.as_str());
            Capability {
                source_file_type: source,
                implemented,
                enabled: available,
                status: if !implemented {
                    CapabilityStatus::Deferred
                } else if available {
                    CapabilityStatus::Enabled
                } else {
                    CapabilityStatus::NotEnabled
                },
            }
        })
        .collect();
        Ok(OperationPage {
            items,
            limit: l,
            offset: o,
        })
    }
}
