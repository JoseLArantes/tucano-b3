//! Shared, read-only analytical contracts for REST and MCP.
mod catalog;
mod governance;
mod operations;
mod quotes;
mod registry;
pub use catalog::*;
pub use governance::*;
pub use operations::*;
pub use quotes::*;
pub use registry::*;

use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Filtro inválido: {0}")]
    Invalid(String),
    #[error("Nenhuma revisão está disponível.")]
    Unavailable,
    #[error("Recurso não encontrado.")]
    NotFound,
    #[error("Fonte/finalidade não autorizada.")]
    Forbidden,
    #[error("Série ambígua; selecione mercado, distribuição e prazo.")]
    Ambiguous,
    #[error("Falha na dependência de consulta.")]
    Database(#[from] sqlx::Error),
}

#[derive(Clone, Copy)]
pub enum QueryPurpose {
    Rest,
    Mcp,
}
impl QueryPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rest => "rest",
            Self::Mcp => "mcp",
        }
    }
}

#[derive(Clone)]
pub struct QueryService {
    pub(crate) pool: PgPool,
}
impl QueryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    /// Bound analytical work while leaving pool capacity for command submission.
    pub(crate) async fn bounded<T>(
        &self,
        work: impl std::future::Future<Output = Result<T, QueryError>>,
    ) -> Result<T, QueryError> {
        static READS: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
        let concurrency = (self.pool.options().get_max_connections() / 2).clamp(1, 4) as usize;
        let semaphore = READS.get_or_init(|| tokio::sync::Semaphore::new(concurrency));
        tokio::time::timeout(std::time::Duration::from_secs(15), async {
            let _permit = semaphore
                .acquire()
                .await
                .map_err(|_| QueryError::Unavailable)?;
            work.await
        })
        .await
        .map_err(|_| QueryError::Unavailable)?
    }
    pub async fn resolve_revision(
        &self,
        revision: Option<Uuid>,
        purpose: QueryPurpose,
    ) -> Result<Uuid, QueryError> {
        let id = match revision {
            Some(id) => {
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM b3_history_revisions WHERE id=$1 AND status='published')").bind(id).fetch_one(&self.pool).await?;
                if !exists {
                    return Err(QueryError::NotFound);
                }
                id
            }
            None => sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT revision_id FROM b3_history_active WHERE singleton=true",
            )
            .fetch_optional(&self.pool)
            .await?
            .flatten()
            .ok_or(QueryError::Unavailable)?,
        };
        // Every selected market and identity source must permit this transport and third parties.
        let eligible: bool = sqlx::query_scalar("WITH files AS (SELECT source_file_id FROM b3_history_revisions WHERE id=$1 UNION SELECT m.source_file_id FROM b3_history_revision_quotes q JOIN b3_source_file_members m ON m.id=q.source_member_id WHERE q.revision_id=$1 UNION SELECT i.source_file_id FROM b3_history_revision_quotes q JOIN b3_instruments i ON i.id=q.instrument_id WHERE q.revision_id=$1) SELECT NOT EXISTS(SELECT 1 FROM files JOIN b3_source_files sf ON sf.id=files.source_file_id CROSS JOIN unnest(ARRAY[$2::text,'third_party_delivery']) p(purpose) WHERE NOT EXISTS(SELECT 1 FROM b3_source_eligibility e WHERE e.source_file_type=sf.source_file_type AND e.channel=sf.acquisition_channel AND e.purpose=p.purpose AND e.status='eligible' AND e.zero_cost AND length(trim(e.evidence_reference))>0 AND e.verified_at IS NOT NULL))")
            .bind(id).bind(purpose.as_str()).fetch_one(&self.pool).await?;
        if !eligible {
            return Err(QueryError::Forbidden);
        }
        Ok(id)
    }
}
