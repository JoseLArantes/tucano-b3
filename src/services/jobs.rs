//! Durable commands shared by transports and executors. No command executes on submission.
mod runs;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db::enums::SourceFileType;
use crate::error::{AppError, Result};

/// Closed administrative commands; acquisition locates publications through the source catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobCommand {
    Ingest {
        /// Official B3 source with a supported record-level date field; no consumer upload is accepted.
        source: SourceFileType,
        /// Inclusive cut on the business date inside each COTAHIST detail record.
        #[schema(example = "2025-01-02")]
        from: NaiveDate,
        /// Inclusive cut on the business date inside each COTAHIST detail record.
        #[schema(example = "2025-01-31")]
        to: NaiveDate,
    },
    WithdrawWindow {
        /// Inclusive first business date to omit from the active historical view.
        #[schema(example = "2025-01-02")]
        from: NaiveDate,
        /// Inclusive last business date to omit from the active historical view.
        #[schema(example = "2025-01-31")]
        to: NaiveDate,
    },
    Reparse {
        source_file_id: Uuid,
    },
    ReplayQuarantine {
        source_file_id: Uuid,
    },
    RebuildIdentity {
        from: NaiveDate,
        to: NaiveDate,
    },
    PublishRevision {
        revision_id: Uuid,
        acknowledge_issues: bool,
    },
    RollbackRevision {
        revision_id: Uuid,
    },
}

impl JobCommand {
    fn scope(&self) -> Result<String> {
        match self {
            Self::Ingest { source, from, to } => {
                validate_dates(*from, *to)?;
                if !source.supports_record_date_cuts() {
                    return Err(AppError::Validation(
                        "Esta família ainda não pode aplicar cortes às datas internas dos registros.".into(),
                    ));
                }
                Ok(format!("ingest:{source}:{from}:{to}"))
            }
            Self::Reparse { source_file_id } | Self::ReplayQuarantine { source_file_id } => {
                Ok(format!("artifact:{source_file_id}"))
            }
            Self::RebuildIdentity { from, to } => {
                validate_dates(*from, *to)?;
                Ok(format!("identity:{from}:{to}"))
            }
            Self::WithdrawWindow { from, to } => {
                validate_dates(*from, *to)?;
                Ok(format!("withdraw:{from}:{to}"))
            }
            Self::PublishRevision { .. } | Self::RollbackRevision { .. } => {
                Ok("publication:cotahist".into())
            }
        }
    }
}

fn validate_dates(from: NaiveDate, to: NaiveDate) -> Result<()> {
    if from > to {
        return Err(AppError::Validation("Intervalo de datas invertido.".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    Maintenance,
    Backfill,
    Rebuild,
}
impl JobPriority {
    fn as_str(self) -> &'static str {
        match self {
            Self::Maintenance => "maintenance",
            Self::Backfill => "backfill",
            Self::Rebuild => "rebuild",
        }
    }
}

/// Maintenance executors never consume heavy jobs, preserving reserved capacity.
pub enum ExecutionLane {
    Maintenance,
    Heavy,
}

/// Fase operacional, separada do estado terminal e da visibilidade da revisão.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobPhase {
    Queued,
    Acquisition,
    Inventory,
    Parsing,
    Staging,
    Sealing,
    Ready,
    Publication,
    Identity,
    Completed,
}
impl JobPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Acquisition => "acquisition",
            Self::Inventory => "inventory",
            Self::Parsing => "parsing",
            Self::Staging => "staging",
            Self::Sealing => "sealing",
            Self::Ready => "ready",
            Self::Publication => "publication",
            Self::Identity => "identity",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobClaim {
    pub id: Uuid,
    pub generation: i64,
    pub owner_id: Uuid,
    pub command: JobCommand,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum JobCompletion {
    Succeeded,
    SucceededWithIssues,
    Cancelled,
}

#[derive(Clone)]
pub struct JobService {
    pool: PgPool,
}

impl JobService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Same key and typed command returns the original job; conflicting requests fail.
    pub async fn submit(
        &self,
        key: &str,
        command: JobCommand,
        priority: JobPriority,
        max_attempts: i32,
    ) -> Result<Uuid> {
        self.submit_linked(key, command, priority, max_attempts, None)
            .await
    }

    async fn submit_linked(
        &self,
        key: &str,
        command: JobCommand,
        priority: JobPriority,
        max_attempts: i32,
        retry_of: Option<Uuid>,
    ) -> Result<Uuid> {
        if key.trim().is_empty() || key.len() > 255 || !(1..=100).contains(&max_attempts) {
            return Err(AppError::Validation(
                "Chave ou limite de tentativas inválido.".into(),
            ));
        }
        let scope = command.scope()?;
        let command = serde_json::to_value(command)?;
        let hash = hex::encode(Sha256::digest(serde_json::to_vec(&(
            command.clone(),
            priority,
            max_attempts,
            retry_of,
        ))?));
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        let inserted: Option<Uuid> = sqlx::query_scalar("INSERT INTO b3_jobs(id,idempotency_key,scope_key,command,command_hash,priority,max_attempts,retry_of) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING RETURNING id")
            .bind(id).bind(key).bind(scope).bind(&command).bind(&hash).bind(priority.as_str()).bind(max_attempts).bind(retry_of).fetch_optional(&mut *tx).await?;
        if let Some(id) = inserted {
            sqlx::query("INSERT INTO b3_sync_runs(id,job_type,status,source_file_type,reference_date_from,reference_date_to) VALUES($1,$2,'queued',$3,$4,$5)").bind(id.to_string()).bind(command.get("operation").and_then(|v|v.as_str()).unwrap_or("unknown")).bind(command.get("source").and_then(|v|v.as_str())).bind(command.get("from").and_then(|v|v.as_str()).and_then(|s|s.parse::<NaiveDate>().ok())).bind(command.get("to").and_then(|v|v.as_str()).and_then(|s|s.parse::<NaiveDate>().ok())).execute(&mut *tx).await?;
            runs::mirror(&mut tx, Some(id)).await?;
            tx.commit().await?;
            return Ok(id);
        }
        let existing = sqlx::query("SELECT id,command_hash FROM b3_jobs WHERE idempotency_key=$1")
            .bind(key)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(row) = existing {
            if row.get::<String, _>("command_hash") == hash {
                tx.commit().await?;
                return Ok(row.get("id"));
            }
        }
        Err(AppError::LockConflict(
            "Pedido incompatível ou escopo já enfileirado.".into(),
        ))
    }

    pub async fn claim(
        &self,
        owner: Uuid,
        lane: ExecutionLane,
        lease_seconds: i32,
    ) -> Result<Option<JobClaim>> {
        if !(1..=3600).contains(&lease_seconds) {
            return Err(AppError::Validation("Lease inválida.".into()));
        }
        let mut tx = self.pool.begin().await?;
        // Exhausted or cancelled abandoned jobs release their durable scope reservations.
        sqlx::query("WITH expired AS (SELECT id FROM b3_jobs WHERE status='running' AND lease_expires_at<=NOW() AND (cancel_requested_at IS NOT NULL OR attempt>=max_attempts) ORDER BY id FOR UPDATE SKIP LOCKED LIMIT 100), closed AS (UPDATE b3_jobs j SET status=CASE WHEN cancel_requested_at IS NOT NULL THEN 'cancelled' ELSE 'failed' END,owner_id=NULL,lease_expires_at=NULL,finished_at=NOW(),error_code=COALESCE(error_code,'LEASE_EXPIRED'),error_message=COALESCE(error_message,'Lease expirada antes da confirmação.') FROM expired e WHERE j.id=e.id RETURNING j.id,j.generation) UPDATE b3_job_attempts a SET outcome='expired',error_code='LEASE_EXPIRED',error_message='Lease expirada antes da confirmação.',finished_at=NOW() FROM closed c WHERE a.job_id=c.id AND a.generation=c.generation AND a.finished_at IS NULL")
            .execute(&mut *tx).await?;
        runs::mirror(&mut tx, None).await?;
        let row = sqlx::query("SELECT id,command FROM b3_jobs WHERE ((status='queued' AND next_attempt_at<=NOW()) OR (status='running' AND lease_expires_at<=NOW())) AND cancel_requested_at IS NULL AND attempt<max_attempts AND ((priority='maintenance')=$1) ORDER BY next_attempt_at,created_at,id FOR UPDATE SKIP LOCKED LIMIT 1")
            .bind(matches!(lane, ExecutionLane::Maintenance)).fetch_optional(&mut *tx).await?;
        let Some(row) = row else {
            tx.commit().await?;
            return Ok(None);
        };
        let id: Uuid = row.get("id");
        let command = serde_json::from_value(row.get("command"))?;
        sqlx::query("UPDATE b3_job_attempts SET outcome='expired',error_code='LEASE_EXPIRED',error_message='Lease expirada antes da confirmação.',finished_at=NOW() WHERE job_id=$1 AND finished_at IS NULL").bind(id).execute(&mut *tx).await?;
        let row = sqlx::query("UPDATE b3_jobs SET status='running',attempt=attempt+1,generation=generation+1,owner_id=$2,heartbeat_at=NOW(),lease_expires_at=NOW()+make_interval(secs=>$3::int),error_code=NULL,error_message=NULL WHERE id=$1 RETURNING generation,lease_expires_at")
            .bind(id).bind(owner).bind(lease_seconds).fetch_one(&mut *tx).await?;
        let generation = row.get("generation");
        sqlx::query("INSERT INTO b3_job_attempts(job_id,generation,owner_id) VALUES($1,$2,$3)")
            .bind(id)
            .bind(generation)
            .bind(owner)
            .execute(&mut *tx)
            .await?;
        let claim = JobClaim {
            id,
            generation,
            owner_id: owner,
            command,
            lease_expires_at: row.get("lease_expires_at"),
        };
        runs::mirror(&mut tx, Some(id)).await?;
        tx.commit().await?;
        Ok(Some(claim))
    }

    /// The returned transaction must also contain the protected data/audit writes.
    /// Its row lock serializes cancellation and claim replacement with confirmation.
    pub async fn begin_confirmation(&self, claim: &JobClaim) -> Result<Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await?;
        Self::check_owner(&mut tx, claim, false).await?;
        Ok(tx)
    }

    /// Acquire a potentially contended publication lock before locking the job row.
    /// This lets heartbeats and a replacement claim proceed while the worker waits;
    /// ownership is then revalidated under the row lock before protected writes.
    pub async fn begin_locked_confirmation(
        &self,
        claim: &JobClaim,
        advisory_lock: i64,
    ) -> Result<Transaction<'_, Postgres>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(advisory_lock)
            .execute(&mut *tx)
            .await?;
        Self::check_owner(&mut tx, claim, false).await?;
        Ok(tx)
    }

    async fn check_owner(
        tx: &mut Transaction<'_, Postgres>,
        claim: &JobClaim,
        allow_cancel: bool,
    ) -> Result<()> {
        let valid: Option<Uuid> = sqlx::query_scalar("SELECT id FROM b3_jobs WHERE id=$1 AND generation=$2 AND owner_id=$3 AND status='running' AND lease_expires_at>clock_timestamp() AND ($4 OR cancel_requested_at IS NULL) FOR UPDATE")
            .bind(claim.id).bind(claim.generation).bind(claim.owner_id).bind(allow_cancel).fetch_optional(&mut **tx).await?;
        if valid.is_none() {
            return Err(AppError::LockConflict(
                "Execução expirada, cancelada ou substituída.".into(),
            ));
        }
        Ok(())
    }

    pub async fn heartbeat(&self, claim: &JobClaim, lease_seconds: i32) -> Result<()> {
        if !(1..=3600).contains(&lease_seconds) {
            return Err(AppError::Validation("Lease inválida.".into()));
        }
        let mut tx = self.begin_confirmation(claim).await?;
        sqlx::query("UPDATE b3_jobs SET heartbeat_at=clock_timestamp(),lease_expires_at=clock_timestamp()+make_interval(secs=>$2::int) WHERE id=$1")
            .bind(claim.id).bind(lease_seconds).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn request_cancel(&self, id: Uuid) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let changed=sqlx::query("UPDATE b3_jobs SET cancel_requested_at=COALESCE(cancel_requested_at,NOW()),status=CASE WHEN status='queued' THEN 'cancelled' ELSE status END,finished_at=CASE WHEN status='queued' THEN NOW() ELSE finished_at END WHERE id=$1 AND status IN ('queued','running')")
            .bind(id).execute(&mut *tx).await?.rows_affected()>0;
        runs::mirror(&mut tx, Some(id)).await?;
        tx.commit().await?;
        Ok(changed)
    }

    /// Commit completion in the same transaction as the final protected writes.
    pub async fn complete(
        tx: &mut Transaction<'_, Postgres>,
        claim: &JobClaim,
        completion: JobCompletion,
    ) -> Result<()> {
        Self::check_owner(tx, claim, matches!(completion, JobCompletion::Cancelled)).await?;
        let status = match completion {
            JobCompletion::Succeeded => "succeeded",
            JobCompletion::SucceededWithIssues => "succeeded_with_issues",
            JobCompletion::Cancelled => "cancelled",
        };
        sqlx::query("UPDATE b3_jobs SET status=$2,owner_id=NULL,lease_expires_at=NULL,finished_at=NOW() WHERE id=$1").bind(claim.id).bind(status).execute(&mut **tx).await?;
        sqlx::query("UPDATE b3_job_attempts SET outcome=$3,finished_at=NOW() WHERE job_id=$1 AND generation=$2").bind(claim.id).bind(claim.generation).bind(status).execute(&mut **tx).await?;
        if matches!(
            completion,
            JobCompletion::Succeeded | JobCompletion::SucceededWithIssues
        ) {
            sqlx::query("UPDATE b3_jobs SET phase='completed' WHERE id=$1")
                .bind(claim.id)
                .execute(&mut **tx)
                .await?;
        }
        runs::mirror(tx, Some(claim.id)).await?;
        Ok(())
    }

    /// Retry retains the previous attempt; caller supplies a non-sensitive error code.
    pub async fn fail(
        &self,
        claim: &JobClaim,
        error_code: &str,
        backoff_seconds: i32,
    ) -> Result<()> {
        self.fail_inner(claim, error_code, None, backoff_seconds)
            .await
    }

    /// A controlled, non-sensitive operator explanation supplied by the executor.
    pub async fn fail_with_message(
        &self,
        claim: &JobClaim,
        error_code: &str,
        message: &str,
        backoff_seconds: i32,
    ) -> Result<()> {
        if message.len() > 1024 || message.contains('\0') {
            return Err(AppError::Validation("Mensagem de falha inválida.".into()));
        }
        self.fail_inner(claim, error_code, Some(message), backoff_seconds)
            .await
    }

    async fn fail_inner(
        &self,
        claim: &JobClaim,
        error_code: &str,
        message: Option<&str>,
        backoff_seconds: i32,
    ) -> Result<()> {
        if error_code.is_empty()
            || error_code.len() > 64
            || !error_code
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b == b'_' || b.is_ascii_digit())
            || !(0..=86400).contains(&backoff_seconds)
        {
            return Err(AppError::Validation(
                "Código de falha ou backoff inválido.".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        Self::check_owner(&mut tx, claim, true).await?;
        let status: String = sqlx::query_scalar("UPDATE b3_jobs SET status=CASE WHEN cancel_requested_at IS NOT NULL THEN 'cancelled' WHEN attempt<max_attempts THEN 'queued' ELSE 'failed' END,owner_id=NULL,lease_expires_at=NULL,error_code=$2,error_message=$4,next_attempt_at=NOW()+make_interval(secs=>$3::int),finished_at=CASE WHEN cancel_requested_at IS NOT NULL OR attempt>=max_attempts THEN NOW() ELSE NULL END WHERE id=$1 RETURNING status")
            .bind(claim.id).bind(error_code).bind(backoff_seconds).bind(message).fetch_one(&mut *tx).await?;
        sqlx::query("UPDATE b3_source_files SET status='failed',error_message=$2 WHERE job_id=$1 AND status NOT IN ('ingested','failed')").bind(claim.id).bind(message.unwrap_or(error_code)).execute(&mut *tx).await?;
        let outcome = if status == "queued" { "retry" } else { &status };
        sqlx::query("UPDATE b3_job_attempts SET outcome=$3,error_code=$4,error_message=$5,finished_at=NOW() WHERE job_id=$1 AND generation=$2").bind(claim.id).bind(claim.generation).bind(outcome).bind(error_code).bind(message).execute(&mut *tx).await?;
        runs::mirror(&mut tx, Some(claim.id)).await?;
        tx.commit().await?;
        Ok(())
    }

    /// A terminal attempt is preserved; retry creates a new tracked command with lineage.
    pub async fn retry(&self, original: Uuid, key: &str) -> Result<Uuid> {
        let row =
            sqlx::query("SELECT command,priority,max_attempts,status FROM b3_jobs WHERE id=$1")
                .bind(original)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::Validation("Execução original não encontrada.".into()))?;
        if !matches!(
            row.get::<String, _>("status").as_str(),
            "failed" | "cancelled"
        ) {
            return Err(AppError::LockConflict(
                "Somente execução falha ou cancelada pode ser repetida.".into(),
            ));
        }
        let command = serde_json::from_value(row.get("command"))?;
        let priority = match row.get::<String, _>("priority").as_str() {
            "maintenance" => JobPriority::Maintenance,
            "backfill" => JobPriority::Backfill,
            "rebuild" => JobPriority::Rebuild,
            _ => return Err(AppError::Validation("Prioridade inválida.".into())),
        };
        self.submit_linked(
            key,
            command,
            priority,
            row.get("max_attempts"),
            Some(original),
        )
        .await
    }

    pub async fn set_phase(&self, claim: &JobClaim, phase: JobPhase) -> Result<()> {
        let mut tx = self.begin_confirmation(claim).await?;
        sqlx::query("UPDATE b3_jobs SET phase=$2 WHERE id=$1")
            .bind(claim.id)
            .bind(phase.as_str())
            .execute(&mut *tx)
            .await?;
        runs::mirror(&mut tx, Some(claim.id)).await?;
        tx.commit().await?;
        Ok(())
    }
}
