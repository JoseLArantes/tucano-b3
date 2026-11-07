use crate::error::Result;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(super) async fn quote_counters(
    tx: &mut Transaction<'_, Postgres>,
    revision: Uuid,
) -> Result<()> {
    // Business counters are accumulated atomically with each bounded staging batch.
    // Sealing only finalizes member totals; rescanning and ranking millions of
    // occurrences here would make publication depend on one unbounded statement.
    sqlx::query(
        "UPDATE b3_source_file_members m SET row_count_raw=rm.detail_read::int,row_count_quarantined=rm.rejected::int,status=CASE WHEN rm.status='complete' THEN 'processed' ELSE 'failed' END,processed_at=NOW() FROM b3_history_revision_members rm WHERE rm.revision_id=$1 AND m.id=rm.source_member_id",
    )
    .bind(revision)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE b3_source_files f SET row_count_raw=c.raw,row_count_inserted=c.inserted,row_count_changed=c.changed,row_count_unchanged=c.unchanged,row_count_duplicate=c.duplicates,row_count_quarantined=c.rejected FROM (SELECT m.source_file_id,COALESCE(sum(m.row_count_raw),0)::int raw,COALESCE(sum(m.row_count_inserted),0)::int inserted,COALESCE(sum(m.row_count_changed),0)::int changed,COALESCE(sum(m.row_count_unchanged),0)::int unchanged,COALESCE(sum(m.row_count_duplicate),0)::bigint duplicates,COALESCE(sum(m.row_count_quarantined),0)::int rejected FROM b3_history_revision_members rm JOIN b3_source_file_members m ON m.id=rm.source_member_id WHERE rm.revision_id=$1 GROUP BY m.source_file_id) c WHERE f.id=c.source_file_id",
    )
    .bind(revision)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) async fn registry_counters(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: Uuid,
) -> Result<()> {
    sqlx::query("WITH previous AS(SELECT DISTINCT ON(r.security_id,r.security_source,r.market_identifier_code) r.* FROM b3_registry_records r JOIN b3_registry_snapshots s ON s.base_snapshot_id=r.snapshot_id WHERE s.id=$1 AND r.security_id IS NOT NULL ORDER BY r.security_id,r.security_source,r.market_identifier_code,r.record_index),ranked AS(SELECT r.source_member_id,r.business_hash,old.business_hash previous_hash,row_number() OVER(PARTITION BY COALESCE(r.security_id,r.source_member_id||':'||r.record_index),r.security_source,r.market_identifier_code ORDER BY m.member_index,r.record_index) rn FROM b3_registry_records r JOIN b3_source_file_members m ON m.id=r.source_member_id LEFT JOIN previous old ON old.security_id=r.security_id AND old.security_source=r.security_source AND old.market_identifier_code IS NOT DISTINCT FROM r.market_identifier_code WHERE r.snapshot_id=$1),counts AS(SELECT source_member_id,count(*)::int accepted,count(*) FILTER(WHERE rn=1 AND previous_hash IS NULL)::int inserted,count(*) FILTER(WHERE rn=1 AND previous_hash<>business_hash)::int changed,count(*) FILTER(WHERE rn=1 AND previous_hash=business_hash)::int unchanged,count(*) FILTER(WHERE rn>1) duplicates FROM ranked GROUP BY source_member_id),rejected AS(SELECT source_member_id,count(*)::int rejected FROM b3_registry_rejections WHERE snapshot_id=$1 GROUP BY source_member_id) UPDATE b3_source_file_members m SET row_count_raw=COALESCE(c.accepted,0)+COALESCE(r.rejected,0),row_count_inserted=COALESCE(c.inserted,0),row_count_changed=COALESCE(c.changed,0),row_count_unchanged=COALESCE(c.unchanged,0),row_count_duplicate=COALESCE(c.duplicates,0),row_count_quarantined=COALESCE(r.rejected,0),status=CASE WHEN rm.structural_valid THEN 'processed' ELSE 'failed' END,processed_at=NOW() FROM b3_registry_members rm LEFT JOIN counts c ON c.source_member_id=rm.source_member_id LEFT JOIN rejected r ON r.source_member_id=rm.source_member_id WHERE rm.snapshot_id=$1 AND m.id=rm.source_member_id").bind(snapshot).execute(&mut **tx).await?;
    sqlx::query("UPDATE b3_source_files f SET row_count_raw=c.raw,row_count_inserted=c.inserted,row_count_changed=c.changed,row_count_unchanged=c.unchanged,row_count_duplicate=c.duplicates,row_count_quarantined=c.rejected FROM (SELECT s.source_file_id,COALESCE(sum(m.row_count_raw),0)::int raw,COALESCE(sum(m.row_count_inserted),0)::int inserted,COALESCE(sum(m.row_count_changed),0)::int changed,COALESCE(sum(m.row_count_unchanged),0)::int unchanged,COALESCE(sum(m.row_count_duplicate),0)::bigint duplicates,COALESCE(sum(m.row_count_quarantined),0)::int rejected FROM b3_registry_snapshots s JOIN b3_registry_members rm ON rm.snapshot_id=s.id JOIN b3_source_file_members m ON m.id=rm.source_member_id WHERE s.id=$1 GROUP BY s.source_file_id) c WHERE f.id=c.source_file_id").bind(snapshot).execute(&mut **tx).await?;
    Ok(())
}
