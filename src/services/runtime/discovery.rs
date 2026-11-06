//! Discovery records observations; it never downloads or parses a dataset.
use super::RuntimeSettings;
use crate::{
    db::enums::SourceFileType,
    error::Result,
    services::{
        artifacts::{ArtifactLimits, ArtifactService},
        eligibility::{require_eligible, SourcePurpose},
    },
};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

pub struct DiscoveryService {
    pool: PgPool,
    settings: RuntimeSettings,
}
impl DiscoveryService {
    pub fn new(pool: PgPool, settings: RuntimeSettings) -> Self {
        Self { pool, settings }
    }

    pub async fn scan(&self, worker_id: Uuid, scheduled: bool, now: DateTime<Utc>) -> Result<()> {
        self.settings.validate()?;
        let market_date = now
            .with_timezone(&chrono_tz::America::Sao_Paulo)
            .date_naive();
        let calendar_known:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_trading_calendar WHERE calendar_date=$1 AND evidence_id IS NOT NULL)").bind(market_date).fetch_one(&self.pool).await?;
        let channels = sqlx::query(
            "SELECT * FROM b3_source_channels WHERE enabled AND discovery_enabled ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        for channel in channels {
            let id: String = channel.get("id");
            let mut outcome = "inconclusive";
            let mut etag = None;
            let mut last_modified = None;
            let mut length = None;
            let source = SourceFileType::from_str(&channel.get::<String, _>("source_file_type"));
            if let Ok(source) = source {
                if require_eligible(&self.pool, source, &id, &[SourcePurpose::Acquisition])
                    .await
                    .is_err()
                {
                    outcome = "ineligible";
                } else if scheduled
                    && !calendar_known
                    && channel.get::<chrono::NaiveDate, _>("period_end") >= market_date
                {
                    outcome = "calendar_unknown";
                } else if channel.get::<String, _>("acquisition_mode") == "local" {
                    // Metadata only; actual hashing/import belongs to the administrative executor.
                    let root = self.settings.raw_root.join("imports");
                    let path =
                        std::path::PathBuf::from(channel.get::<String, _>("local_relative_path"));
                    if !path.is_absolute()
                        && path
                            .components()
                            .all(|p| matches!(p, std::path::Component::Normal(_)))
                    {
                        let base = root.canonicalize();
                        let target = root.join(path).canonicalize();
                        if let (Ok(base), Ok(target)) = (base, target) {
                            if target.starts_with(base) && target.is_file() {
                                let meta = std::fs::metadata(target)?;
                                length = Some(meta.len() as i64);
                                outcome = "available";
                            } else {
                                outcome = "failed";
                            }
                        } else {
                            outcome = "absent";
                        }
                    } else {
                        outcome = "failed";
                    }
                } else {
                    let service = ArtifactService::new(
                        self.settings.raw_root.clone(),
                        self.settings.allowed_hosts.clone(),
                        ArtifactLimits::default(),
                    )?;
                    match service.probe(&channel.get::<String, _>("source_url")).await {
                        Ok(observation) => {
                            outcome = match observation.status {
                                200..=299 => "available",
                                404 => "absent",
                                _ => "inconclusive",
                            };
                            etag = observation.etag;
                            last_modified = observation.last_modified;
                            length = observation.content_length.map(|v| v as i64);
                        }
                        Err(_) => {
                            outcome = "failed";
                        }
                    }
                }
            }
            let mut tx = self.pool.begin().await?;
            sqlx::query("INSERT INTO b3_discovery_checks(id,channel_id,scheduled,outcome,etag,last_modified,content_length) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(Uuid::new_v4()).bind(&id).bind(scheduled).bind(outcome).bind(&etag).bind(&last_modified).bind(length).execute(&mut *tx).await?;
            if outcome == "available" {
                // Acknowledged validators suppress duplicate operational alerts only until
                // periodic revalidation. They never certify equality of business content.
                let acknowledged:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_channel_baselines WHERE channel_id=$1 AND etag IS NOT DISTINCT FROM $2 AND last_modified IS NOT DISTINCT FROM $3 AND content_length IS NOT DISTINCT FROM $4 AND acknowledged_at+make_interval(secs=>$5::int)>NOW())")
                    .bind(&id).bind(&etag).bind(&last_modified).bind(length).bind(channel.get::<i32,_>("revalidation_seconds")).fetch_one(&mut *tx).await?;
                if !acknowledged {
                    sqlx::query("INSERT INTO b3_pending_updates(id,source_file_type,channel_id,remote_etag,remote_last_modified,remote_content_length,status) VALUES($1,$2,$3,$4,$5,$6,'pending') ON CONFLICT DO NOTHING")
                        .bind(Uuid::new_v4().to_string()).bind(channel.get::<String,_>("source_file_type")).bind(id).bind(etag).bind(last_modified).bind(length).execute(&mut *tx).await?;
                }
            }
            tx.commit().await?;
        }
        if scheduled {
            sqlx::query(
                "UPDATE b3_worker_heartbeats SET last_discovery_at=NOW() WHERE worker_id=$1",
            )
            .bind(worker_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    /// Administrative acknowledgement requires a published artifact from this channel.
    /// It closes a discovery alert, never adjudicates conflicting market facts.
    pub async fn acknowledge(&self, pending: Uuid, file: Uuid, reason: &str) -> Result<()> {
        if reason.trim().is_empty() || reason.len() > 1024 {
            return Err(crate::error::AppError::Validation(
                "Motivo obrigatório, limitado a 1024 bytes.".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT * FROM b3_pending_updates WHERE id=$1 FOR UPDATE")
            .bind(pending.to_string())
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| crate::error::AppError::NotFound("Pendência não encontrada.".into()))?;
        if row.get::<String, _>("status") != "pending" {
            return Err(crate::error::AppError::LockConflict(
                "Pendência já tratada.".into(),
            ));
        }
        let channel: Option<String> = row.get("channel_id");
        let channel = channel.ok_or_else(|| {
            crate::error::AppError::Validation("Pendência sem canal homologado.".into())
        })?;
        let published:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_source_files WHERE id=$1 AND acquisition_channel=$2 AND status='ingested')").bind(file.to_string()).bind(&channel).fetch_one(&mut *tx).await?;
        if !published {
            return Err(crate::error::AppError::LockConflict(
                "Baseline exige artefato publicado do mesmo canal.".into(),
            ));
        }
        sqlx::query("INSERT INTO b3_channel_baselines(channel_id,source_file_id,etag,last_modified,content_length,reason) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(channel_id) DO UPDATE SET source_file_id=EXCLUDED.source_file_id,etag=EXCLUDED.etag,last_modified=EXCLUDED.last_modified,content_length=EXCLUDED.content_length,reason=EXCLUDED.reason,acknowledged_at=NOW()")
            .bind(&channel).bind(file.to_string()).bind(row.get::<Option<String>,_>("remote_etag")).bind(row.get::<Option<String>,_>("remote_last_modified")).bind(row.get::<Option<i64>,_>("remote_content_length")).bind(reason).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_pending_updates SET status='resolved',resolution_reason=$2,resolved_source_file_id=$3,handled_at=NOW() WHERE id=$1").bind(pending.to_string()).bind(reason).bind(file.to_string()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
