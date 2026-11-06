//! Heavy work is performed only by claimed administrative jobs.
use super::RuntimeSettings;
use crate::{
    db::{
        enums::SourceFileType,
        history::{
            CandidateSpec, HistoryRepository, LocatedQuote, LocatedRejection, MemberOutcome,
        },
    },
    error::{AppError, Result},
    parsers::cotahist_stream::{parse_cotahist_stream, CotahistEvent},
    services::{
        artifacts::{ArtifactLimits, ArtifactService, RawArtifact},
        eligibility::{require_eligible, SourcePurpose},
        jobs::{JobClaim, JobCommand, JobPhase, JobService},
    },
};
use sqlx::{PgPool, Row};
use std::{fs::File, io::BufReader, path::Path, str::FromStr, time::Instant};
use tracing::{info, warn};
use uuid::Uuid;

pub struct ReleaseExecutor {
    pool: PgPool,
    settings: RuntimeSettings,
}

impl ReleaseExecutor {
    pub fn new(pool: PgPool, settings: RuntimeSettings) -> Self {
        Self { pool, settings }
    }

    pub async fn execute(&self, claim: &JobClaim) -> Result<()> {
        self.settings.validate()?;
        let history = HistoryRepository::new(self.pool.clone());
        let jobs = JobService::new(self.pool.clone());
        match &claim.command {
            JobCommand::Ingest { source, from, to } => {
                let channels = sqlx::query("SELECT * FROM b3_source_channels WHERE enabled AND source_file_type=$1 AND period_start<=$3 AND period_end>=$2 ORDER BY period_start,id")
                    .bind(source.as_str()).bind(from).bind(to).fetch_all(&self.pool).await?;
                if channels.is_empty() {
                    return Err(invalid("Nenhum canal habilitado para o período."));
                }
                for channel in channels {
                    jobs.set_phase(claim, JobPhase::Acquisition).await?;
                    let id: String = channel.get("id");
                    let acquisition_mode: String = channel.get("acquisition_mode");
                    validate_profile(*source, &channel.get::<String, _>("format_profile"))?;
                    require_eligible(
                        &self.pool,
                        *source,
                        &id,
                        &[
                            SourcePurpose::Acquisition,
                            SourcePurpose::RawRetention,
                            SourcePurpose::Processing,
                        ],
                    )
                    .await?;
                    let artifacts = self.artifacts()?;
                    let acquisition_started = Instant::now();
                    let raw = if acquisition_mode == "local" {
                        artifacts
                            .import_local(Path::new(
                                &channel.get::<String, _>("local_relative_path"),
                            ))
                            .await?
                    } else {
                        artifacts
                            .download(&channel.get::<String, _>("source_url"))
                            .await?
                    };
                    info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_type=%source,channel_id=%id,phase="acquisition",acquisition_mode=%acquisition_mode,byte_size=raw.byte_size,sha256=%raw.sha256,elapsed_ms=acquisition_started.elapsed().as_millis() as u64,"Artefato adquirido e finalizado");
                    self.ingest_artifact(
                        claim,
                        *source,
                        &id,
                        &channel.get::<String, _>("publication_key"),
                        channel.get("period_start"),
                        channel.get("period_end"),
                        channel.get("official_revision"),
                        channel.get("replaces_publication"),
                        &channel.get::<String, _>("format_profile"),
                        raw,
                    )
                    .await?;
                }
                Ok(())
            }
            JobCommand::Reparse { source_file_id }
            | JobCommand::ReplayQuarantine { source_file_id } => {
                let row = sqlx::query("SELECT * FROM b3_source_files WHERE id=$1")
                    .bind(source_file_id.to_string())
                    .fetch_optional(&self.pool)
                    .await?
                    .ok_or_else(|| invalid("Arquivo de origem não encontrado."))?;
                let source = SourceFileType::from_str(&row.get::<String, _>("source_file_type"))
                    .map_err(|_| invalid("Família inválida."))?;
                let channel: String = row.get("acquisition_channel");
                validate_profile(source, &row.get::<String, _>("format_profile"))?;
                require_eligible(
                    &self.pool,
                    source,
                    &channel,
                    &[SourcePurpose::RawRetention, SourcePurpose::Processing],
                )
                .await?;
                let raw = RawArtifact {
                    sha256: row.get("sha256"),
                    byte_size: row.get::<i64, _>("byte_size") as u64,
                    path: row.get::<String, _>("local_path").into(),
                };
                // Replay reads only a manifest-owned content-addressed artifact.
                self.ingest_artifact(
                    claim,
                    source,
                    &channel,
                    &row.get::<String, _>("publication_key"),
                    row.get("period_start"),
                    row.get("period_end"),
                    row.get("official_revision"),
                    row.get("replaces_publication"),
                    &row.get::<String, _>("format_profile"),
                    raw,
                )
                .await
            }
            JobCommand::PublishRevision {
                revision_id,
                acknowledge_issues,
            } => {
                let started = Instant::now();
                info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,revision_id=%revision_id,phase="publication","Publicação iniciada");
                let result = history
                    .publish(claim, *revision_id, *acknowledge_issues)
                    .await;
                match result {
                    Ok(()) => {
                        info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,revision_id=%revision_id,phase="publication",elapsed_ms=started.elapsed().as_millis() as u64,"Publicação concluída");
                        Ok(())
                    }
                    Err(error) => {
                        warn!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,revision_id=%revision_id,phase="publication","Publicação não concluída; consulte o estado persistido");
                        Err(error)
                    }
                }
            }
            JobCommand::RollbackRevision { revision_id } => {
                jobs.set_phase(claim, JobPhase::Publication).await?;
                history.rollback(claim, *revision_id).await
            }
            JobCommand::RebuildIdentity { from, to } => {
                jobs.set_phase(claim, JobPhase::Identity).await?;
                history.rebuild_identity(claim, *from, *to).await?;
                let mut tx = jobs.begin_confirmation(claim).await?;
                sqlx::query("INSERT INTO b3_job_outputs(job_id,output_kind,output_id) SELECT $1,'revision',id::text FROM b3_history_revisions WHERE job_id=$1 ON CONFLICT DO NOTHING").bind(claim.id).execute(&mut *tx).await?;
                tx.commit().await?;
                Ok(())
            }
        }
    }

    fn artifacts(&self) -> Result<ArtifactService> {
        ArtifactService::new(
            self.settings.raw_root.clone(),
            self.settings.allowed_hosts.clone(),
            ArtifactLimits::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    async fn ingest_artifact(
        &self,
        claim: &JobClaim,
        source: SourceFileType,
        channel: &str,
        publication_key: &str,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
        official_revision: Option<i64>,
        replaces: bool,
        profile: &str,
        raw: RawArtifact,
    ) -> Result<()> {
        let jobs = JobService::new(self.pool.clone());
        let file_id = Uuid::new_v4().to_string();
        let mut tx = jobs.begin_confirmation(claim).await?;
        sqlx::query("INSERT INTO b3_source_files(id,source_file_type,source_name,local_path,sha256,byte_size,downloaded_at,parser_version,status,acquisition_channel,job_id,job_generation) VALUES($1,$2,$2,$3,$4,$5,NOW(),'0.0.1','downloaded',$6,$7,$8)")
            .bind(&file_id).bind(source.as_str()).bind(raw.path.to_string_lossy().as_ref()).bind(&raw.sha256).bind(raw.byte_size as i64).bind(channel).bind(claim.id).bind(claim.generation).execute(&mut *tx).await?;
        sqlx::query("UPDATE b3_source_files SET period_start=$2,period_end=$3,publication_key=$4,official_revision=$5,replaces_publication=$6,format_profile=$7,reference_date=$8 WHERE id=$1")
            .bind(&file_id).bind(start).bind(end).bind(publication_key).bind(official_revision).bind(replaces).bind(profile).bind(if start==end{Some(start)}else{None}).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO b3_job_outputs(job_id,output_kind,output_id) VALUES($1,'source_file',$2) ON CONFLICT DO NOTHING").bind(claim.id).bind(&file_id).execute(&mut *tx).await?;
        if let JobCommand::Reparse { source_file_id }
        | JobCommand::ReplayQuarantine { source_file_id } = &claim.command
        {
            sqlx::query("UPDATE b3_source_files SET replayed_from_file_id=$2 WHERE id=$1")
                .bind(&file_id)
                .bind(source_file_id.to_string())
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        jobs.set_phase(claim, JobPhase::Inventory).await?;
        let artifacts = self.artifacts()?;
        let inventory_started = Instant::now();
        let mut members = tokio::task::spawn_blocking(move || artifacts.inventory(&raw))
            .await
            .map_err(|_| invalid("Falha na tarefa de inventário."))??;
        for member in &mut members {
            let name = member.name.to_ascii_lowercase();
            let accepted = name == "payload"
                || match source {
                    SourceFileType::CotahistDaily
                    | SourceFileType::CotahistMonthly
                    | SourceFileType::CotahistYearly => name.ends_with(".txt"),
                    SourceFileType::Bvbg028InstrumentsFile => name.ends_with(".xml"),
                    SourceFileType::TradableSecurityList => {
                        if profile.ends_with("_json") {
                            name.ends_with(".json")
                        } else {
                            name.ends_with(".csv")
                        }
                    }
                    _ => false,
                };
            if member.selected && !accepted {
                member.selected = false;
                member.exclusion_reason = Some("FORMAT_NOT_SELECTED".into());
            }
        }
        let selected = members.iter().filter(|m| m.selected).count();
        info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,phase="inventory",member_count=members.len(),selected_member_count=selected,elapsed_ms=inventory_started.elapsed().as_millis() as u64,"Inventário de artefato concluído");
        if selected == 0 {
            return Err(invalid("Artefato sem members processáveis."));
        }
        let mut tx = jobs.begin_confirmation(claim).await?;
        let mut member_ids = Vec::new();
        for m in &members {
            let id = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO b3_source_file_members(id,source_file_id,member_name,member_index,sha256,byte_size,status,parent_index,local_path,selected,exclusion_reason) VALUES($1,$2,$3,$4,$5,$6,'inventoried',$7,$8,$9,$10)")
                .bind(&id).bind(&file_id).bind(&m.name).bind(m.member_index as i32).bind(&m.sha256).bind(m.byte_size as i64).bind(m.parent_index.map(|v|v as i32)).bind(m.path.to_string_lossy().as_ref()).bind(m.selected).bind(&m.exclusion_reason).execute(&mut *tx).await?;
            member_ids.push(id);
        }
        sqlx::query("UPDATE b3_source_files SET member_count=$2 WHERE id=$1")
            .bind(&file_id)
            .bind(members.len() as i32)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        let history = HistoryRepository::new(self.pool.clone());
        let registry = matches!(
            source,
            SourceFileType::Bvbg028InstrumentsFile | SourceFileType::TradableSecurityList
        );
        let revision = if registry {
            if start != end {
                return Err(invalid("Cadastro exige uma data de referência singular."));
            }
            history
                .create_registry_snapshot(claim, &file_id, channel, start, source, selected as i32)
                .await?
        } else {
            history
                .create_candidate(
                    claim,
                    CandidateSpec {
                        source_file_id: file_id.clone(),
                        channel: channel.into(),
                        expected_members: selected as i32,
                        period_start: start,
                        period_end: end,
                        publication_key: publication_key.into(),
                        official_revision,
                        replaces_publication: replaces,
                    },
                )
                .await?
        };
        info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,revision_id=%revision,phase="staging",expected_members=selected,"Revisão candidata criada");
        let mut tx = jobs.begin_confirmation(claim).await?;
        sqlx::query("INSERT INTO b3_job_outputs(job_id,output_kind,output_id) VALUES($1,'revision',$2) ON CONFLICT DO NOTHING").bind(claim.id).bind(revision.to_string()).execute(&mut *tx).await?;
        tx.commit().await?;
        for (member, id) in members.into_iter().zip(member_ids) {
            jobs.set_phase(claim, JobPhase::Parsing).await?;
            if !member.selected {
                continue;
            }
            let member_index = member.member_index;
            let member_bytes = member.byte_size;
            let member_started = Instant::now();
            match source {
                SourceFileType::CotahistDaily
                | SourceFileType::CotahistMonthly
                | SourceFileType::CotahistYearly => {
                    let (sender, mut receiver) = tokio::sync::mpsc::channel(32);
                    let parser = tokio::task::spawn_blocking(move || {
                        parse_cotahist_stream(BufReader::new(File::open(member.path)?), |event| {
                            sender
                                .blocking_send(event)
                                .map_err(|_| invalid("Processamento cancelado."))
                        })
                    });
                    let mut rows = Vec::new();
                    let mut rejected = Vec::new();
                    let mut bytes = 0usize;
                    let mut detail_read = 0i64;
                    let mut rejected_read = 0i64;
                    while let Some(event) = receiver.recv().await {
                        detail_read += 1;
                        match event {
                            CotahistEvent::Valid {
                                row, line_number, ..
                            } => {
                                bytes += 2048;
                                rows.push(LocatedQuote {
                                    record_index: line_number as i64,
                                    row: *row,
                                });
                            }
                            CotahistEvent::Rejected {
                                raw,
                                reason,
                                line_number,
                                byte_offset,
                            } => {
                                rejected_read += 1;
                                bytes += raw.len();
                                rejected.push(LocatedRejection {
                                    record_index: line_number as i64,
                                    byte_offset: Some(byte_offset as i64),
                                    raw_payload: hex::encode(raw),
                                    reason_code: "INVALID_DETAIL".into(),
                                    message: reason,
                                });
                            }
                        }
                        if rows.len() + rejected.len() >= self.settings.batch_records
                            || bytes >= self.settings.batch_bytes
                        {
                            if !rows.is_empty() {
                                history.stage_batch(claim, revision, &id, &rows).await?;
                                rows.clear();
                            }
                            if !rejected.is_empty() {
                                history
                                    .quarantine_batch(claim, revision, &id, &rejected)
                                    .await?;
                                rejected.clear();
                            }
                            bytes = 0;
                        }
                    }
                    if !rows.is_empty() {
                        history.stage_batch(claim, revision, &id, &rows).await?;
                    }
                    if !rejected.is_empty() {
                        history
                            .quarantine_batch(claim, revision, &id, &rejected)
                            .await?;
                    }
                    let parsed = parser
                        .await
                        .map_err(|_| invalid("Tarefa de parsing interrompida."))?;
                    match parsed {
                        Ok(summary) => {
                            history
                                .complete_member(
                                    claim,
                                    revision,
                                    &id,
                                    &MemberOutcome {
                                        detail_read: summary.detail_records as i64,
                                        rejected: summary.rejected_records as i64,
                                        structural_valid: true,
                                    },
                                )
                                .await?;
                            info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,member_id=%id,member_index,revision_id=%revision,phase="parsing",detail_records=summary.detail_records,rejected_records=summary.rejected_records,byte_size=member_bytes,elapsed_ms=member_started.elapsed().as_millis() as u64,"Member COTAHIST processado");
                        }
                        Err(error) => {
                            history
                                .complete_member(
                                    claim,
                                    revision,
                                    &id,
                                    &MemberOutcome {
                                        detail_read,
                                        rejected: rejected_read,
                                        structural_valid: false,
                                    },
                                )
                                .await?;
                            warn!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,member_id=%id,member_index,revision_id=%revision,phase="parsing",detail_records=detail_read,rejected_records=rejected_read,"Member COTAHIST estruturalmente inválido");
                            return Err(error);
                        }
                    }
                }
                SourceFileType::Bvbg028InstrumentsFile | SourceFileType::TradableSecurityList => {
                    info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,member_id=%id,member_index,revision_id=%revision,source_file_type=%source,phase="parsing",byte_size=member_bytes,"Member de cadastro iniciado");
                    super::registry::ingest_member(
                        &history,
                        claim,
                        revision,
                        source,
                        &id,
                        member.path,
                        profile,
                        start,
                        self.settings.batch_records,
                        self.settings.batch_bytes,
                    )
                    .await?;
                    info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,member_id=%id,member_index,revision_id=%revision,source_file_type=%source,phase="staging",byte_size=member_bytes,elapsed_ms=member_started.elapsed().as_millis() as u64,"Member de cadastro processado");
                }
                _ => return Err(invalid("Família não habilitada no lançamento.")),
            }
        }
        if registry {
            jobs.set_phase(claim, JobPhase::Sealing).await?;
            history.seal_registry_snapshot(claim, revision).await?;
        } else {
            jobs.set_phase(claim, JobPhase::Sealing).await?;
            history.seal_candidate(claim, revision).await?;
        }
        info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,source_file_id=%file_id,revision_id=%revision,phase="sealing",selected_member_count=selected,"Candidato validado para publicação explícita");
        let mut tx = jobs.begin_confirmation(claim).await?;
        sqlx::query("UPDATE b3_source_files SET status='parsed',parsed_at=NOW() WHERE id=$1")
            .bind(file_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}
fn invalid(message: &str) -> AppError {
    AppError::Validation(message.into())
}

fn validate_profile(source: SourceFileType, profile: &str) -> Result<()> {
    let accepted = match source {
        SourceFileType::CotahistDaily
        | SourceFileType::CotahistMonthly
        | SourceFileType::CotahistYearly => profile == "cotahist_latin1",
        SourceFileType::Bvbg028InstrumentsFile => profile == "bvbg028_iso20022",
        SourceFileType::TradableSecurityList => matches!(
            profile,
            "tradable_csv_semicolon_utf8"
                | "tradable_b3_security_definition_v101_csv"
                | "tradable_b3_security_definition_v101_json"
        ),
        _ => false,
    };
    if !accepted {
        return Err(invalid("Perfil de formato não homologado/habilitado."));
    }
    Ok(())
}
