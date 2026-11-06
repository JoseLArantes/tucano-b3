use super::HistoryRepository;
use crate::{
    error::{AppError, Result},
    services::jobs::{JobClaim, JobService},
};
use chrono::NaiveDate;
use uuid::Uuid;

impl HistoryRepository {
    /// Rebuild creates a candidate relationship view; market versions and their hashes remain immutable.
    pub async fn rebuild_identity(
        &self,
        claim: &JobClaim,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<()> {
        if from > to {
            return Err(AppError::Validation(
                "Intervalo de identidade inválido.".into(),
            ));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let base: Option<Uuid> =
            sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
                .fetch_one(&mut *tx)
                .await?;
        let base = base
            .ok_or_else(|| AppError::Validation("Nenhuma revisão histórica publicada.".into()))?;
        let revision = Uuid::new_v4();
        sqlx::query("INSERT INTO b3_history_revisions(id,job_id,base_revision_id,source_file_id,channel,period_start,period_end,publication_key,official_revision,replaces_publication,status,expected_members,rejected,conflicts) SELECT $1,$2,id,source_file_id,channel,period_start,period_end,publication_key,official_revision,FALSE,'ready',expected_members,rejected,conflicts FROM b3_history_revisions WHERE id=$3").bind(revision).bind(claim.id).bind(base).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_history_revisions SET revision_kind='identity' WHERE id=$1")
            .bind(revision)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_history_revision_members SELECT $1,source_member_id,status,detail_read,rejected FROM b3_history_revision_members WHERE revision_id=$2").bind(revision).bind(base).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_history_revision_quotes SELECT $1,fact_id,version_id,source_member_id,record_index,instrument_id,resolution_status,selected_publication_key,selected_official_revision FROM b3_history_revision_quotes WHERE revision_id=$2").bind(revision).bind(base).execute(&mut *tx).await?;
        // Inclusive evidence windows. Unknown dates/market mappings never widen a match.
        sqlx::query("WITH evidence AS (SELECT q.fact_id,array_agg(DISTINCT i.instrument_id) FILTER(WHERE i.identifier_type='ticker') ticker_ids,array_agg(DISTINCT i.instrument_id) FILTER(WHERE i.identifier_type='isin') isin_ids FROM b3_history_revision_quotes q JOIN b3_cotahist_facts f ON f.id=q.fact_id JOIN b3_cotahist_versions v ON v.id=q.version_id LEFT JOIN b3_instrument_identifiers i ON i.market_code IS NOT DISTINCT FROM f.market_type AND i.valid_from<=f.trade_date AND i.valid_to>=f.trade_date AND ((i.identifier_type='ticker' AND i.identifier_value=f.ticker_symbol) OR (i.identifier_type='isin' AND i.identifier_value=v.isin)) WHERE q.revision_id=$1 AND f.trade_date BETWEEN $2 AND $3 GROUP BY q.fact_id), candidates AS (SELECT fact_id,CASE WHEN cardinality(ticker_ids)>0 AND cardinality(isin_ids)>0 THEN ARRAY(SELECT unnest(ticker_ids) INTERSECT SELECT unnest(isin_ids)) ELSE COALESCE(isin_ids,ticker_ids,ARRAY[]::varchar[]) END ids,(cardinality(ticker_ids)>0 AND cardinality(isin_ids)>0) both_present FROM evidence) UPDATE b3_history_revision_quotes q SET instrument_id=CASE WHEN cardinality(c.ids)=1 THEN c.ids[1] ELSE NULL END,resolution_status=CASE WHEN cardinality(c.ids)=1 THEN 'resolved' WHEN cardinality(c.ids)>1 OR c.both_present THEN 'ambiguous' ELSE 'unresolved' END FROM candidates c WHERE q.revision_id=$1 AND q.fact_id=c.fact_id").bind(revision).bind(from).bind(to).execute(&mut *tx).await?;
        // Marking sourcefile timestamps here would falsely report market ingestion/change.
        tx.commit().await?;
        Ok(())
    }
}
