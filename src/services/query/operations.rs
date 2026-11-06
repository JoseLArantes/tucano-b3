use super::{QueryError, QueryService};
use crate::services::jobs::{JobCommand, JobPriority};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;
use uuid::Uuid;

/// Estados fechados da execução administrativa.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    SucceededWithIssues,
    Failed,
    Cancelled,
}
/// Fase operacional independente do resultado terminal.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

/// Acompanhamento seguro de um trabalho durável; sucesso de submissão não é ingestão concluída.
#[derive(Debug, Serialize, ToSchema)]
pub struct JobView {
    /// UUID estável do trabalho.
    pub id: Uuid,
    /// Trabalho terminal de origem quando esta execução é um retry explícito.
    pub retry_of: Option<Uuid>,
    /// Comando tipado submetido pelo operador.
    pub command: JobCommand,
    /// Classe de capacidade reservada.
    pub priority: JobPriority,
    /// Estado atual da execução.
    pub status: JobStatus,
    /// Fase efetivamente persistida pelo executor.
    pub phase: JobPhase,
    /// Número de tentativas efetivamente iniciadas.
    pub attempt: i32,
    /// Máximo de tentativas configurado no pedido.
    pub max_attempts: i32,
    /// Geração de claim usada para fencing.
    pub generation: i64,
    /// Última confirmação de vida do executor, em UTC.
    pub heartbeat_at: Option<DateTime<Utc>>,
    /// Solicitação durável de cancelamento, em UTC.
    pub cancel_requested_at: Option<DateTime<Utc>>,
    /// Momento em que novo retry pode ser reclamado.
    pub next_attempt_at: DateTime<Utc>,
    /// Código seguro da última falha, sem SQL ou tokens.
    pub error_code: Option<String>,
    /// Explicação controlada para recuperação, sem SQL ou credenciais.
    pub error_message: Option<String>,
    /// Instante de criação do trabalho.
    pub created_at: DateTime<Utc>,
    /// Instante de encerramento, quando terminal.
    pub finished_at: Option<DateTime<Utc>>,
    /// Recursos efetivamente produzidos e persistidos pelo executor.
    pub outputs: Vec<JobOutputView>,
}

/// Referência fechada ao resultado persistido de uma operação.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JobOutputView {
    /// Tipo do resultado.
    pub kind: JobOutputKind,
    /// ID do recurso produzido; não é caminho privado.
    pub id: String,
}
/// Tipos de recurso produzidos por jobs.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobOutputKind {
    SourceFile,
    Revision,
    IdentityRebuild,
}

const JOB_COLUMNS:&str="SELECT j.*,(SELECT COALESCE(jsonb_agg(jsonb_build_object('kind',o.output_kind,'id',o.output_id) ORDER BY o.output_kind,o.output_id),'[]'::jsonb) FROM b3_job_outputs o WHERE o.job_id=j.id) AS outputs FROM b3_jobs j";

impl QueryService {
    pub async fn job(&self, id: Uuid) -> Result<JobView, QueryError> {
        let row = sqlx::query(&format!("{JOB_COLUMNS} WHERE j.id=$1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(QueryError::NotFound)?;
        job_view(row)
    }
    pub async fn jobs(
        &self,
        filters: super::ListFilters,
    ) -> Result<super::OperationPage<JobView>, QueryError> {
        let (l, o) = filters.validate()?;
        let rows = sqlx::query(&format!(
            "{JOB_COLUMNS} ORDER BY j.created_at,j.id LIMIT $1 OFFSET $2"
        ))
        .bind(l)
        .bind(o)
        .fetch_all(&self.pool)
        .await?;
        Ok(super::OperationPage {
            items: rows
                .into_iter()
                .map(job_view)
                .collect::<Result<Vec<_>, _>>()?,
            limit: l,
            offset: o,
        })
    }
}
fn job_view(row: sqlx::postgres::PgRow) -> Result<JobView, QueryError> {
    let command =
        serde_json::from_value(row.get("command")).map_err(|_| QueryError::Unavailable)?;
    let priority = serde_json::from_value(serde_json::Value::String(row.get("priority")))
        .map_err(|_| QueryError::Unavailable)?;
    let status = serde_json::from_value(serde_json::Value::String(row.get("status")))
        .map_err(|_| QueryError::Unavailable)?;
    let phase = serde_json::from_value(serde_json::Value::String(row.get("phase")))
        .map_err(|_| QueryError::Unavailable)?;
    Ok(JobView {
        id: row.get("id"),
        retry_of: row.get("retry_of"),
        command,
        priority,
        status,
        phase,
        attempt: row.get("attempt"),
        max_attempts: row.get("max_attempts"),
        generation: row.get("generation"),
        heartbeat_at: row.get("heartbeat_at"),
        cancel_requested_at: row.get("cancel_requested_at"),
        next_attempt_at: row.get("next_attempt_at"),
        error_code: row.get("error_code"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        finished_at: row.get("finished_at"),
        outputs: serde_json::from_value(row.get("outputs")).map_err(|_| QueryError::Unavailable)?,
    })
}
