//! Ingestion models for tucano-b3.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Resposta de operações de ingestão.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IngestResponse {
    pub task_id: String,
    pub files_processed: usize,
    pub total_records_parsed: usize,
    pub total_records_new: usize,
    pub total_records_skipped: usize,
    pub success: bool,
    pub message: String,
}

/// Request body for folder ingestion.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct FolderIngestRequest {
    /// Absolute path to the folder containing COTAHIST files
    pub caminho: String,
}

/// Corpo opcional de `POST /ingest/bucket`.
///
/// O bucket, o endpoint e as credenciais vêm das variáveis `TUCANO_S3_*`.
/// A ingestão só ocorre quando este endpoint é chamado.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BucketIngestRequest {
    /// Prefixo das chaves no bucket. Se omitido, usa `TUCANO_S3_PREFIX`.
    /// Exemplo: `cotahist/`.
    #[serde(default)]
    pub prefix: Option<String>,
}

/// Request body para upload de arquivo COTAHIST (Swagger UI / OpenAPI).
#[derive(Debug, ToSchema)]
pub struct UploadFileRequest {
    /// Arquivo ZIP contendo arquivos COTAHIST (.TXT) da B3
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}
