//! Registry snapshots preserve source-specific values and never manufacture historical validity.
use super::HistoryRepository;
use crate::{
    db::enums::SourceFileType,
    error::{AppError, Result},
    parsers::{
        bvbg_028::Bvbg028Row,
        canonical::{canonicalize_string, stable_row_hash},
        tradable_list::TradableSecurityRow,
    },
    services::{
        eligibility::{require_eligible, SourcePurpose},
        jobs::{JobClaim, JobCompletion, JobService},
    },
};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{Postgres, QueryBuilder, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Default, Serialize)]
struct RegistryRecord {
    pub security_id: Option<String>,
    pub security_source: String,
    pub market_identifier_code: Option<String>,
    pub ticker_symbol: Option<String>,
    pub isin: Option<String>,
    pub asset: Option<String>,
    pub asset_description: Option<String>,
    pub market_code: Option<String>,
    pub market_name: Option<String>,
    pub segment_code: Option<String>,
    pub segment_name: Option<String>,
    pub instrument_type: Option<String>,
    pub security_category: Option<String>,
    pub cfi_code: Option<String>,
    pub instrument_status: Option<String>,
    pub maturity_date: Option<NaiveDate>,
    pub tradable_indicator: Option<bool>,
    pub trading_start_date: Option<NaiveDate>,
    pub trading_end_date: Option<NaiveDate>,
}
impl HistoryRepository {
    pub async fn quarantine_registry_batch(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rejected: &[super::LocatedRejection],
    ) -> Result<()> {
        if rejected.len() > 1000
            || rejected
                .iter()
                .any(|r| r.record_index < 1 || r.raw_payload.len() > 4096)
        {
            return Err(AppError::Validation(
                "Lote de rejeições cadastrais inválido.".into(),
            ));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_registry_snapshots s JOIN b3_source_file_members m ON m.source_file_id=s.source_file_id WHERE s.id=$1 AND s.status='draft' AND m.id=$2)").bind(snapshot).bind(member).fetch_one(&mut *tx).await?;
        if !valid {
            return Err(AppError::Validation("Fotografia/member inválido.".into()));
        }
        if !rejected.is_empty() {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_registry_rejections(snapshot_id,source_member_id,record_index,byte_offset,reason_code,message,raw_payload) ");
            q.push_values(rejected, |mut b, r| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(r.record_index)
                    .push_bind(r.byte_offset)
                    .push_bind(&r.reason_code)
                    .push_bind(&r.message)
                    .push_bind(&r.raw_payload);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("INSERT INTO b3_quarantine_records(id,source_file_id,source_member_name,source_file_type,reference_date,source_line_number,reason_code,error_message,raw_payload,registry_snapshot_id,source_record_index,byte_offset) SELECT gen_random_uuid()::text,s.source_file_id,m.member_name,s.source_file_type,s.reference_date,CASE WHEN r.record_index<=2147483647 THEN r.record_index::int END,r.reason_code,r.message,jsonb_build_object('raw',r.raw_payload),$1,r.record_index,r.byte_offset FROM b3_registry_rejections r JOIN b3_registry_snapshots s ON s.id=r.snapshot_id JOIN b3_source_file_members m ON m.id=r.source_member_id WHERE r.snapshot_id=$1 AND r.source_member_id=$2 ON CONFLICT DO NOTHING").bind(snapshot).bind(member).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_registry_snapshots SET rejected=(SELECT count(*) FROM b3_registry_rejections WHERE snapshot_id=$1) WHERE id=$1").bind(snapshot).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn create_registry_snapshot(
        &self,
        claim: &JobClaim,
        source_file_id: &str,
        channel: &str,
        reference_date: NaiveDate,
        source: SourceFileType,
        expected_members: i32,
    ) -> Result<Uuid> {
        if !matches!(
            source,
            SourceFileType::Bvbg028InstrumentsFile | SourceFileType::TradableSecurityList
        ) || expected_members < 1
            || channel.trim().is_empty()
        {
            return Err(AppError::Validation(
                "Fonte/escopo cadastral inválido.".into(),
            ));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let id = Uuid::new_v4();
        let inventory_matches:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_source_files s WHERE s.id=$1 AND s.source_file_type=$2 AND s.acquisition_channel=$3 AND (SELECT count(*) FROM b3_source_file_members m WHERE m.source_file_id=s.id AND m.selected)=$4)").bind(source_file_id).bind(source.as_str()).bind(channel).bind(i64::from(expected_members)).fetch_one(&mut *tx).await?;
        if !inventory_matches {
            return Err(AppError::Validation(
                "Inventário/canal cadastral não corresponde ao artefato.".into(),
            ));
        }
        let base: Option<Uuid> = sqlx::query_scalar(
            "SELECT snapshot_id FROM b3_registry_active WHERE source_file_type=$1",
        )
        .bind(source.as_str())
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO b3_registry_snapshots(id,source_file_id,source_file_type,channel,reference_date,base_snapshot_id,expected_members,job_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)").bind(id).bind(source_file_id).bind(source.as_str()).bind(channel).bind(reference_date).bind(base).bind(expected_members).bind(claim.id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }
    pub async fn stage_registry_batch(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rows: &[(i64, Bvbg028Row)],
    ) -> Result<()> {
        self.instrument_projection_batch(claim, snapshot, member, rows, "BVBG_028_INSTRUMENTS_FILE")
            .await
    }

    pub async fn stage_security_definition_projection_batch(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rows: &[(i64, Bvbg028Row)],
    ) -> Result<()> {
        self.instrument_projection_batch(claim, snapshot, member, rows, "TRADABLE_SECURITY_LIST")
            .await
    }

    async fn instrument_projection_batch(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rows: &[(i64, Bvbg028Row)],
        family: &str,
    ) -> Result<()> {
        let rows: Vec<_> = rows
            .iter()
            .map(|(index, r)| {
                (
                    *index,
                    RegistryRecord {
                        security_id: canonicalize_string(Some(&r.security_id)),
                        security_source: canonicalize_string(r.security_source.as_deref())
                            .unwrap_or_else(|| {
                                if family == "TRADABLE_SECURITY_LIST" {
                                    "TRADABLE_UNHOMOLOGATED".into()
                                } else {
                                    "BVBG_028_UNHOMOLOGATED".into()
                                }
                            }),
                        market_identifier_code: r.market_identifier_code.clone(),
                        ticker_symbol: r.ticker_symbol.clone(),
                        isin: r.isin.clone(),
                        asset: r.asset.clone(),
                        asset_description: r.asset_description.clone(),
                        market_code: r.market_code.clone(),
                        market_name: r.market_name.clone(),
                        segment_code: r.segment_code.clone(),
                        segment_name: r.segment_name.clone(),
                        instrument_type: r.instrument_type.clone(),
                        security_category: r.security_category.clone(),
                        cfi_code: r.cfi_code.clone(),
                        instrument_status: r.instrument_status.clone(),
                        maturity_date: r.maturity_date,
                        ..Default::default()
                    },
                )
            })
            .collect();
        self.registry_batch(claim, snapshot, member, &rows, family)
            .await
    }
    pub async fn stage_tradable_batch(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rows: &[(i64, TradableSecurityRow)],
    ) -> Result<()> {
        let rows: Vec<_> = rows
            .iter()
            .map(|(index, r)| {
                (
                    *index,
                    RegistryRecord {
                        security_id: r.security_id.clone(),
                        security_source: "TRADABLE_UNHOMOLOGATED".into(),
                        market_identifier_code: r.market_code.clone(),
                        ticker_symbol: r.ticker_symbol.clone(),
                        isin: r.isin.clone(),
                        asset: None,
                        asset_description: None,
                        market_code: r.market_code.clone(),
                        market_name: None,
                        segment_code: r.segment_code.clone(),
                        segment_name: None,
                        instrument_type: None,
                        security_category: None,
                        cfi_code: None,
                        instrument_status: None,
                        maturity_date: None,
                        tradable_indicator: r.tradable_indicator,
                        trading_start_date: r.trading_start_date,
                        trading_end_date: r.trading_end_date,
                    },
                )
            })
            .collect();
        self.registry_batch(claim, snapshot, member, &rows, "TRADABLE_SECURITY_LIST")
            .await
    }
    async fn registry_batch(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        rows: &[(i64, RegistryRecord)],
        family: &str,
    ) -> Result<()> {
        if rows.len() > 1000 || rows.iter().any(|(i, _)| *i < 1) {
            return Err(AppError::Validation("Lote cadastral inválido.".into()));
        }
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let allowed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_registry_snapshots s JOIN b3_source_file_members m ON m.source_file_id=s.source_file_id WHERE s.id=$1 AND s.status='draft' AND s.source_file_type=$3 AND m.id=$2)").bind(snapshot).bind(member).bind(family).fetch_one(&mut *tx).await?;
        if !allowed {
            return Err(AppError::Validation(
                "Fotografia/member cadastral inválido.".into(),
            ));
        }
        let rows: Vec<_> = rows
            .iter()
            .map(|(i, r)| {
                let fields: BTreeMap<String, serde_json::Value> =
                    serde_json::from_value(serde_json::to_value(r)?)?;
                Ok((*i, r, stable_row_hash(&fields)))
            })
            .collect::<Result<_>>()?;
        if !rows.is_empty() {
            let mut q=QueryBuilder::<Postgres>::new("INSERT INTO b3_registry_records(snapshot_id,source_member_id,record_index,security_id,security_source,market_identifier_code,ticker_symbol,isin,asset,asset_description,market_code,market_name,segment_code,segment_name,instrument_type,security_category,cfi_code,instrument_status,maturity_date,tradable_indicator,trading_start_date,trading_end_date,business_hash) ");
            q.push_values(rows, |mut b, (index, r, hash)| {
                b.push_bind(snapshot)
                    .push_bind(member)
                    .push_bind(index)
                    .push_bind(&r.security_id)
                    .push_bind(&r.security_source)
                    .push_bind(&r.market_identifier_code)
                    .push_bind(&r.ticker_symbol)
                    .push_bind(&r.isin)
                    .push_bind(&r.asset)
                    .push_bind(&r.asset_description)
                    .push_bind(&r.market_code)
                    .push_bind(&r.market_name)
                    .push_bind(&r.segment_code)
                    .push_bind(&r.segment_name)
                    .push_bind(&r.instrument_type)
                    .push_bind(&r.security_category)
                    .push_bind(&r.cfi_code)
                    .push_bind(&r.instrument_status)
                    .push_bind(r.maturity_date)
                    .push_bind(r.tradable_indicator)
                    .push_bind(r.trading_start_date)
                    .push_bind(r.trading_end_date)
                    .push_bind(hash);
            });
            q.push(" ON CONFLICT DO NOTHING")
                .build()
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn complete_registry_member(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        member: &str,
        structural_valid: bool,
    ) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_registry_snapshots s JOIN b3_source_file_members m ON m.source_file_id=s.source_file_id WHERE s.id=$1 AND s.status='draft' AND m.id=$2)").bind(snapshot).bind(member).fetch_one(&mut *tx).await?;
        if !valid {
            return Err(AppError::Validation("Fotografia/member inválido.".into()));
        }
        sqlx::query("INSERT INTO b3_registry_members(snapshot_id,source_member_id,structural_valid) VALUES($1,$2,$3) ON CONFLICT(snapshot_id,source_member_id) DO UPDATE SET structural_valid=EXCLUDED.structural_valid").bind(snapshot).bind(member).bind(structural_valid).execute(&mut *tx).await?;
        if !structural_valid {
            sqlx::query("UPDATE b3_registry_snapshots SET status='invalid' WHERE id=$1")
                .bind(snapshot)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn seal_registry_snapshot(&self, claim: &JobClaim, snapshot: Uuid) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        let collisions:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_registry_records WHERE snapshot_id=$1 AND security_id IS NOT NULL GROUP BY security_id,security_source,market_identifier_code HAVING count(DISTINCT business_hash)>1)").bind(snapshot).fetch_one(&mut *tx).await?;
        if collisions {
            return Err(AppError::Validation(
                "Identificador cadastral colide com conteúdos distintos na mesma fotografia."
                    .into(),
            ));
        }
        let updated=sqlx::query("UPDATE b3_registry_snapshots s SET status='ready' WHERE id=$1 AND status='draft' AND expected_members=(SELECT count(*) FROM b3_registry_members m WHERE m.snapshot_id=s.id AND m.structural_valid) AND NOT EXISTS(SELECT 1 FROM b3_registry_members m WHERE m.snapshot_id=s.id AND NOT m.structural_valid)").bind(snapshot).execute(&mut *tx).await?.rows_affected();
        if updated == 0 {
            return Err(AppError::Validation(
                "Fotografia cadastral incompleta ou inválida.".into(),
            ));
        }
        super::counters::registry_counters(&mut tx, snapshot).await?;
        tx.commit().await?;
        Ok(())
    }
    pub(crate) async fn registry_exists(&self, snapshot: Uuid) -> Result<bool> {
        Ok(
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_registry_snapshots WHERE id=$1)")
                .bind(snapshot)
                .fetch_one(&self.pool)
                .await?,
        )
    }
    pub async fn publish_registry_snapshot(
        &self,
        claim: &JobClaim,
        snapshot: Uuid,
        acknowledge_issues: bool,
    ) -> Result<()> {
        let spec = sqlx::query("SELECT * FROM b3_registry_snapshots WHERE id=$1")
            .bind(snapshot)
            .fetch_one(&self.pool)
            .await?;
        let family: String = spec.get("source_file_type");
        let source: SourceFileType = family.parse().map_err(AppError::Validation)?;
        require_eligible(
            &self.pool,
            source,
            &spec.get::<String, _>("channel"),
            &[
                SourcePurpose::Processing,
                SourcePurpose::RawRetention,
                SourcePurpose::Rest,
                SourcePurpose::Mcp,
                SourcePurpose::ThirdPartyDelivery,
            ],
        )
        .await?;
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(7324031002)")
            .execute(&mut *tx)
            .await?;
        let eligible:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM unnest(ARRAY['processing','raw_retention','rest','mcp','third_party_delivery']) p WHERE NOT EXISTS(SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=$1 AND e.channel=$2 AND e.purpose=p AND e.status='eligible' AND e.zero_cost AND e.verified_at IS NOT NULL AND length(trim(e.evidence_reference))>0))").bind(&family).bind(spec.get::<String,_>("channel")).fetch_one(&mut *tx).await?;
        if !eligible {
            return Err(AppError::Validation(
                "Canal cadastral sem elegibilidade comprovada.".into(),
            ));
        }
        let active: Option<Uuid> = sqlx::query_scalar(
            "SELECT snapshot_id FROM b3_registry_active WHERE source_file_type=$1 FOR UPDATE",
        )
        .bind(&family)
        .fetch_optional(&mut *tx)
        .await?;
        if active == Some(snapshot) {
            JobService::complete(&mut tx, claim, JobCompletion::Succeeded).await?;
            tx.commit().await?;
            return Ok(());
        }
        if spec.get::<String, _>("status") != "ready"
            || spec.get::<Option<Uuid>, _>("base_snapshot_id") != active
        {
            return Err(AppError::LockConflict(
                "Fotografia não publicável ou base cadastral alterada.".into(),
            ));
        }
        if spec.get::<i64, _>("rejected") > 0 && !acknowledge_issues {
            return Err(AppError::Validation(
                "Rejeições cadastrais exigem reconhecimento explícito.".into(),
            ));
        }
        // Namespace fallback is explicitly source-specific; raw numeric coincidence never merges sources.
        sqlx::query("INSERT INTO b3_instruments(id,security_id,security_source,market_identifier_code,asset,asset_description,market_code,market_name,segment_code,segment_name,instrument_type,security_category,cfi_code,instrument_status,maturity_date,source_file_id,source_member_name,source_line_number,source_row_hash) SELECT DISTINCT ON(r.security_id,r.security_source,r.market_identifier_code) gen_random_uuid()::text,r.security_id,r.security_source,r.market_identifier_code,r.asset,r.asset_description,r.market_code,r.market_name,r.segment_code,r.segment_name,r.instrument_type,r.security_category,r.cfi_code,r.instrument_status,r.maturity_date,s.source_file_id,m.member_name,r.record_index::int,r.business_hash FROM b3_registry_records r JOIN b3_registry_snapshots s ON s.id=r.snapshot_id JOIN b3_source_file_members m ON m.id=r.source_member_id WHERE r.snapshot_id=$1 AND r.security_id IS NOT NULL ORDER BY r.security_id,r.security_source,r.market_identifier_code,r.record_index ON CONFLICT(security_id,security_source,market_identifier_code) DO NOTHING").bind(snapshot).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_registry_records r SET instrument_id=i.id FROM b3_instruments i WHERE r.snapshot_id=$1 AND i.security_id=r.security_id AND i.security_source=r.security_source AND i.market_identifier_code IS NOT DISTINCT FROM r.market_identifier_code").bind(snapshot).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_field_changes(id,source_file_id,table_name,record_pk,field_name,old_value_text,new_value_text,registry_snapshot_id) SELECT DISTINCT ON(n.instrument_id,nf.key) gen_random_uuid()::text,s.source_file_id,'b3_registry_records',n.instrument_id,nf.key,old_field.value #>> '{}',nf.value #>> '{}',$1 FROM b3_registry_records n JOIN b3_registry_snapshots s ON s.id=n.snapshot_id JOIN b3_registry_records o ON o.snapshot_id=$2 AND o.security_id=n.security_id AND o.security_source=n.security_source AND o.market_identifier_code IS NOT DISTINCT FROM n.market_identifier_code CROSS JOIN LATERAL jsonb_each(to_jsonb(n)-ARRAY['snapshot_id','source_member_id','record_index','business_hash','instrument_id']) nf JOIN LATERAL jsonb_each(to_jsonb(o)) old_field ON old_field.key=nf.key WHERE n.snapshot_id=$1 AND n.instrument_id IS NOT NULL AND nf.value IS DISTINCT FROM old_field.value ORDER BY n.instrument_id,nf.key,n.record_index").bind(snapshot).bind(active).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_instrument_identifiers(id,instrument_id,identifier_type,identifier_value,market_code,valid_from,valid_to,source_file_id,source_member_name,source_line_number,source_row_hash) SELECT gen_random_uuid()::text,r.instrument_id,v.kind,v.value,r.market_code,s.reference_date,s.reference_date,s.source_file_id,m.member_name,r.record_index::int,r.business_hash FROM b3_registry_records r JOIN b3_registry_snapshots s ON s.id=r.snapshot_id JOIN b3_source_file_members m ON m.id=r.source_member_id CROSS JOIN LATERAL(VALUES('ticker',upper(trim(r.ticker_symbol))),('isin',upper(trim(r.isin))),('security_id',r.security_id)) v(kind,value) WHERE r.snapshot_id=$1 AND r.instrument_id IS NOT NULL AND v.value IS NOT NULL AND length(trim(v.value))>0 ON CONFLICT DO NOTHING").bind(snapshot).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_registry_active(source_file_type,snapshot_id) VALUES($1,$2) ON CONFLICT(source_file_type) DO UPDATE SET snapshot_id=EXCLUDED.snapshot_id").bind(&family).bind(snapshot).execute(&mut *tx).await?;
        sqlx::query(
            "UPDATE b3_registry_snapshots SET status='published',published_at=NOW() WHERE id=$1",
        )
        .bind(snapshot)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO b3_registry_publications(snapshot_id,previous_snapshot_id,job_id,action) VALUES($1,$2,$3,'publish')").bind(snapshot).bind(active).bind(claim.id).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_source_files SET status='ingested',ingested_at=NOW(),row_count_raw=((SELECT count(*) FROM b3_registry_records WHERE snapshot_id=$1)+(SELECT count(*) FROM b3_registry_rejections WHERE snapshot_id=$1))::int,row_count_quarantined=(SELECT count(*)::int FROM b3_registry_rejections WHERE snapshot_id=$1) WHERE id=$2").bind(snapshot).bind(spec.get::<String,_>("source_file_id")).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_quarantine_records old SET resolution_status='reprocessed',reprocessed_at=NOW() FROM b3_source_files current,b3_registry_snapshots s,b3_source_file_members m,b3_registry_records r WHERE s.id=$1 AND current.id=s.source_file_id AND old.source_file_id=current.replayed_from_file_id AND m.source_file_id=current.id AND m.member_name=old.source_member_name AND r.snapshot_id=s.id AND r.source_member_id=m.id AND r.record_index=old.source_record_index AND old.resolution_status='quarantined'").bind(snapshot).execute(&mut *tx).await?;
        JobService::complete(&mut tx, claim, JobCompletion::Succeeded).await?;
        tx.commit().await?;
        Ok(())
    }
    pub async fn rollback_registry_snapshot(&self, claim: &JobClaim, snapshot: Uuid) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let mut tx = jobs.begin_confirmation(claim).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(7324031002)")
            .execute(&mut *tx)
            .await?;
        let family: Option<String> = sqlx::query_scalar(
            "SELECT source_file_type FROM b3_registry_snapshots WHERE id=$1 AND status='published'",
        )
        .bind(snapshot)
        .fetch_optional(&mut *tx)
        .await?;
        let family =
            family.ok_or_else(|| AppError::Validation("Fotografia nunca publicada.".into()))?;
        let active: Option<Uuid> = sqlx::query_scalar(
            "SELECT snapshot_id FROM b3_registry_active WHERE source_file_type=$1 FOR UPDATE",
        )
        .bind(&family)
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query("UPDATE b3_registry_active SET snapshot_id=$2 WHERE source_file_type=$1")
            .bind(&family)
            .bind(snapshot)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO b3_registry_publications(snapshot_id,previous_snapshot_id,job_id,action) VALUES($1,$2,$3,'rollback')").bind(snapshot).bind(active).bind(claim.id).execute(&mut *tx).await?;
        JobService::complete(&mut tx, claim, JobCompletion::Succeeded).await?;
        tx.commit().await?;
        Ok(())
    }
}
