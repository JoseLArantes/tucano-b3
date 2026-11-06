use crate::error::Result;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Keep operational runs, source provenance and counters in the protected transaction.
pub(super) async fn mirror(tx: &mut Transaction<'_, Postgres>, id: Option<Uuid>) -> Result<()> {
    sqlx::query(r#"
 UPDATE b3_sync_runs s
 SET status=j.status,
     started_at=CASE WHEN j.attempt>0 THEN COALESCE(s.started_at,j.heartbeat_at) ELSE s.started_at END,
     finished_at=j.finished_at, updated_at=NOW(),
     source_file_ids=files.ids,
     stats_json=jsonb_build_object('phase',j.phase,'attempt',j.attempt,'generation',j.generation,
       'error_code',j.error_code,'error_message',j.error_message,'inserted',files.inserted,
       'changed',files.changed,'unchanged',files.unchanged,'duplicate_in_scope',files.duplicates,
       'quarantined',files.quarantined)
 FROM b3_jobs j
 LEFT JOIN LATERAL (
   SELECT COALESCE(jsonb_agg(f.id ORDER BY f.id),'[]'::jsonb) ids,
     CASE WHEN count(*)=0 THEN 0 WHEN count(f.row_count_inserted)=count(*) THEN sum(f.row_count_inserted) END inserted,
     CASE WHEN count(*)=0 THEN 0 WHEN count(f.row_count_changed)=count(*) THEN sum(f.row_count_changed) END changed,
     CASE WHEN count(*)=0 THEN 0 WHEN count(f.row_count_unchanged)=count(*) THEN sum(f.row_count_unchanged) END unchanged,
     COALESCE(sum(f.row_count_duplicate),0) duplicates,
     COALESCE(sum(f.row_count_quarantined),0) quarantined
   FROM b3_source_files f
   WHERE f.job_id=j.id
     OR EXISTS(SELECT 1 FROM b3_history_publications p JOIN b3_history_revisions r ON r.id=p.revision_id WHERE p.job_id=j.id AND r.source_file_id=f.id)
     OR EXISTS(SELECT 1 FROM b3_registry_publications p JOIN b3_registry_snapshots r ON r.id=p.snapshot_id WHERE p.job_id=j.id AND r.source_file_id=f.id)
 ) files ON TRUE
 WHERE s.id=j.id::text AND ($1::uuid IS NULL OR j.id=$1)
   AND ($1::uuid IS NOT NULL OR s.status IS DISTINCT FROM j.status)
 "#).bind(id).execute(&mut **tx).await?;
    Ok(())
}
