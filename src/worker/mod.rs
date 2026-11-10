//! Durable executors reserve an independent lane for regular maintenance.
use crate::{
    error::Result,
    services::{
        jobs::{ExecutionLane, JobCompletion, JobService},
        runtime::{discovery::DiscoveryService, executor::ReleaseExecutor, RuntimeSettings},
    },
};
use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use uuid::Uuid;

pub struct B3Worker {
    pool: PgPool,
    settings: RuntimeSettings,
}
impl B3Worker {
    pub fn new(pool: PgPool, settings: RuntimeSettings) -> Self {
        Self { pool, settings }
    }

    pub async fn run_once(&self, lane: ExecutionLane, owner: Uuid) -> Result<bool> {
        let lane_name = match lane {
            ExecutionLane::Maintenance => "maintenance",
            ExecutionLane::Heavy => "heavy",
        };
        sqlx::query("INSERT INTO b3_worker_heartbeats(worker_id,lane) VALUES($1,$2) ON CONFLICT(worker_id) DO UPDATE SET heartbeat_at=NOW(),lane=EXCLUDED.lane").bind(owner).bind(lane_name).execute(&self.pool).await?;
        let jobs = JobService::new(self.pool.clone());
        let Some(claim) = jobs.claim(owner, lane, self.settings.lease_seconds).await? else {
            return Ok(false);
        };
        info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,lane=lane_name,"Execução administrativa iniciada");
        let executor = ReleaseExecutor::new(self.pool.clone(), self.settings.clone());
        let heartbeats = async {
            let mut timer = interval(Duration::from_secs(
                (self.settings.lease_seconds / 3).max(1) as u64,
            ));
            loop {
                timer.tick().await;
                match jobs.heartbeat(&claim, self.settings.lease_seconds).await {
                    Ok(()) => {}
                    Err(error @ crate::error::AppError::LockConflict(_)) => return Err(error),
                    Err(error) => {
                        warn!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,lane=lane_name,error=%error,"Heartbeat temporariamente indisponível; a execução continua e tentará novamente");
                    }
                }
                if let Err(error) = sqlx::query(
                    "UPDATE b3_worker_heartbeats SET heartbeat_at=NOW() WHERE worker_id=$1",
                )
                .bind(owner)
                .execute(&self.pool)
                .await
                {
                    warn!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,lane=lane_name,error=%error,"Heartbeat do worker indisponível; será atualizado na próxima tentativa");
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), crate::error::AppError>(())
        };
        let result =
            tokio::select! {result=executor.execute(&claim)=>result,result=heartbeats=>result};
        let terminal: bool = sqlx::query_scalar(
            "SELECT status IN ('succeeded','succeeded_with_issues') FROM b3_jobs WHERE id=$1",
        )
        .bind(claim.id)
        .fetch_one(&self.pool)
        .await?;
        if terminal {
            return Ok(true);
        }
        match result {
            Ok(()) => {
                let status: String = sqlx::query_scalar("SELECT status FROM b3_jobs WHERE id=$1")
                    .bind(claim.id)
                    .fetch_one(&self.pool)
                    .await?;
                if status == "running" {
                    let mut tx = jobs.begin_confirmation(&claim).await?;
                    JobService::complete(&mut tx, &claim, JobCompletion::Succeeded).await?;
                    tx.commit().await?;
                }
                info!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,lane=lane_name,"Execução administrativa concluída");
            }
            Err(error) => {
                warn!(job_id=%claim.id,run_id=%claim.owner_id,generation=claim.generation,lane=lane_name,"Execução interrompida; consulte estado e tentativa");
                let (code, message) = match &error {
                    crate::error::AppError::Layout(message) => {
                        ("STRUCTURAL_INVALID", message.as_str())
                    }
                    crate::error::AppError::Validation(message) => {
                        ("VALIDATION_FAILED", message.as_str())
                    }
                    crate::error::AppError::LockConflict(_) => (
                        "OWNERSHIP_OR_SCOPE_CONFLICT",
                        "Execução cancelada, expirada ou revisão substituída.",
                    ),
                    crate::error::AppError::Io(_) => (
                        "ARTIFACT_IO_FAILED",
                        "Artefato ausente ou não recuperável; verificar manifesto e armazenamento.",
                    ),
                    _ => (
                        "EXECUTION_FAILED",
                        "Falha de infraestrutura; verificar dependências e retomar a execução.",
                    ),
                };
                let message = message.chars().take(240).collect::<String>();
                let _ = jobs.fail_with_message(&claim, code, &message, 30).await;
            }
        }
        Ok(true)
    }

    pub async fn run_loop(&self) -> Result<()> {
        let regular = Uuid::new_v4();
        let heavy = Uuid::new_v4();
        let maintenance = async {
            loop {
                match self.run_once(ExecutionLane::Maintenance, regular).await {
                    Ok(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                    Err(error) => {
                        warn!(lane="maintenance",error=%error,"Worker indisponível temporariamente; nova tentativa em cinco segundos");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), crate::error::AppError>(())
        };
        let heavy_work = async {
            loop {
                match self.run_once(ExecutionLane::Heavy, heavy).await {
                    Ok(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                    Err(error) => {
                        warn!(lane="heavy",error=%error,"Worker indisponível temporariamente; nova tentativa em cinco segundos");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            #[allow(unreachable_code)]
            Ok::<(), crate::error::AppError>(())
        };
        let discovery = async {
            let scanner = DiscoveryService::new(self.pool.clone(), self.settings.clone());
            loop {
                if std::env::var("B3_DISCOVERY_ENABLED").as_deref() == Ok("true") {
                    if let Err(error) = scanner.scan(regular, true, chrono::Utc::now()).await {
                        warn!(error=%error,"Varredura agendada falhou temporariamente; será repetida no próximo ciclo");
                    }
                }
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), crate::error::AppError>(())
        };
        tokio::select! {result=maintenance=>result,result=heavy_work=>result,result=discovery=>result,_=tokio::signal::ctrl_c()=>Ok(())}
    }
}
