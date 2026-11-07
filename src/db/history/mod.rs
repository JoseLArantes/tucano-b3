//! Typed historical facts and atomic publication. Draft writes never change active visibility.
mod counters;
mod identity;
mod registry;
mod security_definition;
mod staging;
pub use staging::{BatchStats, LocatedQuote, LocatedRejection, MemberOutcome};

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

const CANDIDATE_CARRY_BATCH_SIZE: i64 = 25_000;

use crate::db::enums::SourceFileType;
use crate::{
    error::{AppError, Result},
    services::{
        eligibility::{require_eligible, SourcePurpose},
        jobs::{JobClaim, JobService},
    },
};

#[derive(Clone)]
pub struct HistoryRepository {
    pub(crate) pool: PgPool,
}

pub struct CandidateSpec {
    pub source_file_id: String,
    pub channel: String,
    pub expected_members: i32,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub publication_key: String,
    pub official_revision: Option<i64>,
    pub replaces_publication: bool,
}

#[derive(Debug, Serialize)]
pub struct RevisionInfo {
    pub id: Uuid,
    pub base_revision_id: Option<Uuid>,
    pub source_file_id: String,
    pub channel: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub status: String,
    pub expected_members: i32,
    pub completed_members: i64,
    pub rejected: i64,
    pub conflicts: i64,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

impl HistoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn active_revision(&self) -> Result<Option<Uuid>> {
        Ok(
            sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
                .fetch_one(&self.pool)
                .await?,
        )
    }

    pub async fn create_candidate(&self, claim: &JobClaim, spec: CandidateSpec) -> Result<Uuid> {
        if spec.expected_members < 1
            || spec.period_start > spec.period_end
            || spec.publication_key.trim().is_empty()
            || spec.channel.trim().is_empty()
        {
            return Err(AppError::Validation("Escopo de revisão inválido.".into()));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let inventory_matches:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_source_files s WHERE s.id=$1 AND s.source_file_type IN ('COTAHIST_YEARLY','COTAHIST_MONTHLY','COTAHIST_DAILY') AND s.acquisition_channel=$2 AND (SELECT count(*) FROM b3_source_file_members m WHERE m.source_file_id=s.id AND m.selected)=$3)").bind(&spec.source_file_id).bind(&spec.channel).bind(i64::from(spec.expected_members)).fetch_one(&mut *tx).await?;
        if !inventory_matches {
            return Err(AppError::Validation(
                "Canal, família ou inventário da revisão não corresponde ao artefato.".into(),
            ));
        }
        let existing = sqlx::query("SELECT id,base_revision_id,period_start,period_end FROM b3_history_revisions WHERE job_id=$1 AND revision_kind='ingest' AND status='draft' FOR UPDATE")
            .bind(claim.id)
            .fetch_optional(&mut *tx)
            .await?;
        let (id, base) = if let Some(existing) = existing {
            let id: Uuid = existing.get("id");
            let base: Option<Uuid> = existing.get("base_revision_id");
            if existing.get::<NaiveDate, _>("period_start") != spec.period_start
                || existing.get::<NaiveDate, _>("period_end") != spec.period_end
            {
                return Err(AppError::Validation(
                    "Todos os artefatos do job devem pertencer ao mesmo intervalo solicitado."
                        .into(),
                ));
            }
            let attached: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_history_revision_sources WHERE revision_id=$1 AND source_file_id=$2)")
                .bind(id)
                .bind(&spec.source_file_id)
                .fetch_one(&mut *tx)
                .await?;
            if !attached {
                let equivalent_source: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_history_revision_sources rs JOIN b3_source_files prior ON prior.id=rs.source_file_id JOIN b3_source_files incoming ON incoming.id=$2 WHERE rs.revision_id=$1 AND prior.source_file_type=incoming.source_file_type AND prior.acquisition_channel=incoming.acquisition_channel AND prior.sha256=incoming.sha256 AND prior.publication_key IS NOT DISTINCT FROM incoming.publication_key AND prior.official_revision IS NOT DISTINCT FROM incoming.official_revision AND prior.replaces_publication=incoming.replaces_publication AND prior.format_profile=incoming.format_profile AND prior.requested_from IS NOT DISTINCT FROM incoming.requested_from AND prior.requested_to IS NOT DISTINCT FROM incoming.requested_to)")
                    .bind(id)
                    .bind(&spec.source_file_id)
                    .fetch_one(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO b3_history_revision_sources(revision_id,source_file_id,requested_from,requested_to) VALUES($1,$2,$3,$4)")
                    .bind(id)
                    .bind(&spec.source_file_id)
                    .bind(spec.period_start)
                    .bind(spec.period_end)
                    .execute(&mut *tx)
                    .await?;
                if !equivalent_source {
                    sqlx::query(
                        "UPDATE b3_history_revisions SET expected_members=expected_members+$2 WHERE id=$1",
                    )
                    .bind(id)
                    .bind(spec.expected_members)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            tx.commit().await?;
            (id, base)
        } else {
            let base: Option<Uuid> =
                sqlx::query_scalar("SELECT revision_id FROM b3_history_active WHERE singleton")
                    .fetch_one(&mut *tx)
                    .await?;
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO b3_history_revisions(id,base_revision_id,source_file_id,channel,period_start,period_end,publication_key,official_revision,replaces_publication,expected_members,job_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
                .bind(id).bind(base).bind(&spec.source_file_id).bind(spec.channel).bind(spec.period_start).bind(spec.period_end).bind(spec.publication_key).bind(spec.official_revision).bind(spec.replaces_publication).bind(spec.expected_members).bind(claim.id).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO b3_history_revision_sources(revision_id,source_file_id,requested_from,requested_to) VALUES($1,$2,$3,$4)")
                .bind(id)
                .bind(&spec.source_file_id)
                .bind(spec.period_start)
                .bind(spec.period_end)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            (id, base)
        };
        if let Some(base) = base {
            self.carry_forward_candidate(claim, id, base).await?;
        }
        Ok(id)
    }

    /// Copy a base revision in restartable batches so lease heartbeats and
    /// cancellation can acquire the job row between batches.
    async fn carry_forward_candidate(
        &self,
        claim: &JobClaim,
        revision: Uuid,
        base: Uuid,
    ) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let mut cursor: Option<Uuid> = None;
        loop {
            let mut tx = jobs.begin_confirmation(claim).await?;
            Self::draft(&mut tx, revision).await?;
            let next: Option<Uuid> = sqlx::query_scalar(
                "WITH batch AS MATERIALIZED (SELECT fact_id,version_id,source_member_id,record_index,instrument_id,resolution_status,selected_publication_key,selected_official_revision FROM b3_history_revision_quotes WHERE revision_id=$2 AND ($3::uuid IS NULL OR fact_id>$3) ORDER BY fact_id LIMIT $4), copied AS (INSERT INTO b3_history_revision_quotes(revision_id,fact_id,version_id,source_member_id,record_index,instrument_id,resolution_status,selected_publication_key,selected_official_revision) SELECT $1,fact_id,version_id,source_member_id,record_index,instrument_id,resolution_status,selected_publication_key,selected_official_revision FROM batch ON CONFLICT(revision_id,fact_id) DO NOTHING RETURNING fact_id) SELECT fact_id FROM batch ORDER BY fact_id DESC LIMIT 1",
            )
            .bind(revision)
            .bind(base)
            .bind(cursor)
            .bind(CANDIDATE_CARRY_BATCH_SIZE)
            .fetch_optional(&mut *tx)
            .await?;
            tx.commit().await?;
            match next {
                Some(next) if cursor != Some(next) => cursor = Some(next),
                _ => break,
            }
        }
        Ok(())
    }

    pub(crate) async fn draft(
        tx: &mut Transaction<'_, Postgres>,
        revision: Uuid,
    ) -> Result<sqlx::postgres::PgRow> {
        sqlx::query("SELECT * FROM b3_history_revisions WHERE id=$1 AND status='draft' FOR UPDATE")
            .bind(revision)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| AppError::Validation("Revisão não está em preparação.".into()))
    }

    pub(crate) async fn check_selected_eligibility(
        tx: &mut Transaction<'_, Postgres>,
        revision: Uuid,
    ) -> Result<()> {
        let allowed:bool=sqlx::query_scalar("WITH channels AS (SELECT DISTINCT f.source_file_type,f.acquisition_channel FROM b3_history_revision_quotes q JOIN b3_source_file_members m ON m.id=q.source_member_id JOIN b3_source_files f ON f.id=m.source_file_id WHERE q.revision_id=$1 UNION SELECT DISTINCT f.source_file_type,f.acquisition_channel FROM b3_history_revision_sources rs JOIN b3_source_files f ON f.id=rs.source_file_id WHERE rs.revision_id=$1) SELECT NOT EXISTS(SELECT 1 FROM channels c CROSS JOIN unnest(ARRAY['processing','raw_retention']) purpose WHERE NOT EXISTS(SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=c.source_file_type AND e.channel=c.acquisition_channel AND e.purpose=purpose AND e.status='eligible' AND e.zero_cost AND e.verified_at IS NOT NULL AND length(trim(e.evidence_reference))>0))").bind(revision).fetch_one(&mut **tx).await?;
        if !allowed {
            return Err(AppError::Validation(
                "Revisão contém canal/finalidade sem elegibilidade comprovada.".into(),
            ));
        }
        Ok(())
    }

    pub async fn seal_candidate(&self, claim: &JobClaim, revision: Uuid) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let spec = Self::draft(&mut tx, revision).await?;
        let expected: i32 = spec.get("expected_members");
        let (completed, invalid, rejected): (i64,i64,i64)=sqlx::query_as("SELECT count(*) FILTER(WHERE status='complete'),count(*) FILTER(WHERE status='invalid'),COALESCE(sum(rejected),0)::bigint FROM b3_history_revision_members WHERE revision_id=$1").bind(revision).fetch_one(&mut *tx).await?;
        let conflicts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM b3_history_conflicts WHERE revision_id=$1")
                .bind(revision)
                .fetch_one(&mut *tx)
                .await?;
        if invalid > 0 {
            sqlx::query("UPDATE b3_history_revisions SET status='invalid',rejected=$2,conflicts=$3 WHERE id=$1").bind(revision).bind(rejected).bind(conflicts).execute(&mut *tx).await?;
            tx.commit().await?;
            return Err(AppError::Layout(
                "Revisão com member estruturalmente inválido.".into(),
            ));
        }
        if completed != i64::from(expected) {
            return Err(AppError::Validation(
                "Inventário de members obrigatório ainda incompleto.".into(),
            ));
        }
        // A complete, explicitly ordered source replacement may remove only facts
        // from its publication and within the requested business-date cut.
        if rejected == 0 {
            sqlx::query("DELETE FROM b3_history_revision_quotes q USING b3_cotahist_facts f,b3_history_revisions r WHERE q.revision_id=$1 AND q.fact_id=f.id AND r.id=q.revision_id AND f.trade_date BETWEEN r.period_start AND r.period_end AND EXISTS(SELECT 1 FROM b3_history_revision_sources rs JOIN b3_source_files incoming ON incoming.id=rs.source_file_id WHERE rs.revision_id=r.id AND incoming.replaces_publication AND incoming.official_revision IS NOT NULL AND incoming.publication_key=q.selected_publication_key AND incoming.official_revision>COALESCE(q.selected_official_revision,-1) AND f.trade_date BETWEEN GREATEST(incoming.period_start,r.period_start) AND LEAST(incoming.period_end,r.period_end) AND NOT EXISTS(SELECT 1 FROM b3_cotahist_occurrences o JOIN b3_source_file_members m ON m.id=o.source_member_id JOIN b3_source_files observed ON observed.id=m.source_file_id JOIN b3_cotahist_versions v ON v.id=o.version_id WHERE o.revision_id=r.id AND v.fact_id=q.fact_id AND observed.publication_key=incoming.publication_key AND observed.official_revision=incoming.official_revision))")
                .bind(revision).execute(&mut *tx).await?;
        }
        sqlx::query(
            "UPDATE b3_history_revisions SET status='ready',rejected=$2,conflicts=$3 WHERE id=$1",
        )
        .bind(revision)
        .bind(rejected)
        .bind(conflicts)
        .execute(&mut *tx)
        .await?;
        counters::quote_counters(&mut tx, revision).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn publish(
        &self,
        claim: &JobClaim,
        revision: Uuid,
        acknowledge_issues: bool,
    ) -> Result<()> {
        if self.registry_exists(revision).await? {
            return self
                .publish_registry_snapshot(claim, revision, acknowledge_issues)
                .await;
        }
        let info=sqlx::query("SELECT sf.source_file_type,r.channel,r.revision_kind FROM b3_history_revisions r JOIN b3_source_files sf ON sf.id=r.source_file_id WHERE r.id=$1").bind(revision).fetch_one(&self.pool).await?;
        let withdrawal_only = info.get::<String, _>("revision_kind") == "withdrawal";
        if !withdrawal_only {
            let source: SourceFileType = info
                .get::<String, _>("source_file_type")
                .parse()
                .map_err(AppError::Validation)?;
            require_eligible(
                &self.pool,
                source,
                &info.get::<String, _>("channel"),
                &[SourcePurpose::Processing, SourcePurpose::RawRetention],
            )
            .await?;
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_locked_confirmation(claim, 7324031001).await?;
        if !withdrawal_only {
            Self::check_selected_eligibility(&mut tx, revision).await?;
        }
        let active: Option<Uuid> = sqlx::query_scalar(
            "SELECT revision_id FROM b3_history_active WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let row = sqlx::query("SELECT r.*,r.revision_kind='identity' AS identity_only,r.revision_kind='withdrawal' AS withdrawal_only FROM b3_history_revisions r WHERE r.id=$1 FOR UPDATE OF r")
            .bind(revision)
            .fetch_one(&mut *tx)
            .await?;
        if active == Some(revision) {
            tx.commit().await?;
            return Ok(());
        }
        if row.get::<String, _>("status") != "ready"
            || row.get::<i64, _>("conflicts") > 0
            || (row.get::<i64, _>("rejected") > 0 && !acknowledge_issues)
        {
            return Err(AppError::Validation(
                "Revisão não publicável ou problemas não reconhecidos.".into(),
            ));
        }
        if row.get::<Option<Uuid>, _>("base_revision_id") != active {
            return Err(AppError::LockConflict(
                "Revisão ativa mudou; preparar nova candidata.".into(),
            ));
        }
        sqlx::query("UPDATE b3_history_active SET revision_id=$1 WHERE singleton")
            .bind(revision)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE b3_history_revisions SET status='published',published_at=NOW() WHERE id=$1",
        )
        .bind(revision)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO b3_history_publications(revision_id,previous_revision_id,job_id,action) VALUES($1,$2,$3,'publish')").bind(revision).bind(active).bind(claim.id).execute(&mut *tx).await?;
        if !row.get::<bool, _>("identity_only") && !row.get::<bool, _>("withdrawal_only") {
            sqlx::query("UPDATE b3_source_files f SET status='ingested',ingested_at=NOW(),row_count_raw=COALESCE(c.raw,0),row_count_quarantined=COALESCE(c.rejected,0) FROM (SELECT m.source_file_id,sum(rm.detail_read)::int raw,sum(rm.rejected)::int rejected FROM b3_history_revision_members rm JOIN b3_source_file_members m ON m.id=rm.source_member_id WHERE rm.revision_id=$1 GROUP BY m.source_file_id) c WHERE f.id=c.source_file_id")
                .bind(revision).execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE b3_quarantine_records old SET resolution_status='reprocessed',reprocessed_at=NOW() FROM b3_source_files current,b3_source_file_members m,b3_cotahist_occurrences o,b3_cotahist_versions v,b3_history_revision_quotes selected WHERE m.source_file_id=current.id AND old.source_file_id=current.replayed_from_file_id AND m.member_name=old.source_member_name AND o.revision_id=$1 AND o.source_member_id=m.id AND o.record_index=old.source_record_index AND v.id=o.version_id AND selected.revision_id=$1 AND selected.fact_id=v.fact_id AND selected.version_id=v.id AND old.resolution_status='quarantined'").bind(revision).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn rollback(&self, claim: &JobClaim, revision: Uuid) -> Result<()> {
        if self.registry_exists(revision).await? {
            return self.rollback_registry_snapshot(claim, revision).await;
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_locked_confirmation(claim, 7324031001).await?;
        let active: Option<Uuid> = sqlx::query_scalar(
            "SELECT revision_id FROM b3_history_active WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM b3_history_revisions WHERE id=$1 AND status='published')",
        )
        .bind(revision)
        .fetch_one(&mut *tx)
        .await?;
        if !exists {
            return Err(AppError::Validation(
                "Revisão nunca publicada não pode ser restaurada.".into(),
            ));
        }
        Self::check_selected_eligibility(&mut tx, revision).await?;
        sqlx::query("UPDATE b3_history_active SET revision_id=$1 WHERE singleton")
            .bind(revision)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_history_publications(revision_id,previous_revision_id,job_id,action) VALUES($1,$2,$3,'rollback')").bind(revision).bind(active).bind(claim.id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Publish a new view that omits facts in an inclusive business-date window.
    /// Immutable artifacts, facts, versions, occurrences, and prior revisions remain intact.
    pub async fn withdraw_window(
        &self,
        claim: &JobClaim,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<()> {
        if from > to {
            return Err(AppError::Validation(
                "Intervalo de remoção inválido.".into(),
            ));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_locked_confirmation(claim, 7324031001).await?;
        let active: Option<Uuid> = sqlx::query_scalar(
            "SELECT revision_id FROM b3_history_active WHERE singleton FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let Some(active) = active else {
            tx.commit().await?;
            return Ok(());
        };
        let revision = Uuid::new_v4();
        sqlx::query("INSERT INTO b3_history_revisions(id,job_id,base_revision_id,source_file_id,channel,period_start,period_end,publication_key,official_revision,replaces_publication,status,expected_members,rejected,conflicts,revision_kind) SELECT $1,$2,id,source_file_id,channel,$3,$4,publication_key,official_revision,FALSE,'ready',expected_members,0,0,'withdrawal' FROM b3_history_revisions WHERE id=$5")
            .bind(revision)
            .bind(claim.id)
            .bind(from)
            .bind(to)
            .bind(active)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_history_revision_members(revision_id,source_member_id,status,detail_read,rejected,out_of_scope) SELECT $1,source_member_id,status,detail_read,rejected,out_of_scope FROM b3_history_revision_members WHERE revision_id=$2")
            .bind(revision)
            .bind(active)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_history_revision_sources(revision_id,source_file_id,requested_from,requested_to) SELECT $1,source_file_id,$2,$3 FROM b3_history_revision_sources WHERE revision_id=$4 ON CONFLICT DO NOTHING")
            .bind(revision)
            .bind(from)
            .bind(to)
            .bind(active)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_history_revision_quotes SELECT $1,q.fact_id,q.version_id,q.source_member_id,q.record_index,q.instrument_id,q.resolution_status,q.selected_publication_key,q.selected_official_revision FROM b3_history_revision_quotes q JOIN b3_cotahist_facts f ON f.id=q.fact_id WHERE q.revision_id=$2 AND f.trade_date NOT BETWEEN $3 AND $4")
            .bind(revision)
            .bind(active)
            .bind(from)
            .bind(to)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_job_outputs(job_id,output_kind,output_id) VALUES($1,'revision',$2) ON CONFLICT DO NOTHING")
            .bind(claim.id)
            .bind(revision.to_string())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.publish(claim, revision, false).await
    }

    pub async fn revision(&self, id: Uuid) -> Result<Option<RevisionInfo>> {
        let row=sqlx::query("SELECT r.*,(SELECT count(*) FROM b3_history_revision_members m WHERE m.revision_id=r.id AND m.status='complete') AS completed_members FROM b3_history_revisions r WHERE r.id=$1").bind(id).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| RevisionInfo {
            id: r.get("id"),
            base_revision_id: r.get("base_revision_id"),
            source_file_id: r.get("source_file_id"),
            channel: r.get("channel"),
            period_start: r.get("period_start"),
            period_end: r.get("period_end"),
            status: r.get("status"),
            expected_members: r.get("expected_members"),
            completed_members: r.get("completed_members"),
            rejected: r.get("rejected"),
            conflicts: r.get("conflicts"),
            created_at: r.get("created_at"),
            published_at: r.get("published_at"),
        }))
    }
}
