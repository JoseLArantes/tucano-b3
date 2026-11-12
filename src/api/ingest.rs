//! Ingestion API endpoints.
//!
//! POST /api/v1/ingest/upload  — Upload ZIP file
//! POST /api/v1/ingest/folder  — Ingest from local folder
//! POST /api/v1/ingest/bucket  — Ingest COTAHIST ZIPs from S3-compatible storage

use axum::extract::{DefaultBodyLimit, Multipart, State};
use axum::routing::post;
use axum::{Json, Router};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::api::health::AppState;
use crate::error::{AppError, AppResult};
use crate::ingest::object_storage::{self, S3ObjectStorage};
use crate::ingest::service::IngestService;
use crate::ingest::zip_handler::MAX_ZIP_SIZE;
use crate::models::envelope::ApiResponse;
pub use crate::models::ingest::{
    BucketIngestRequest, FolderIngestRequest, IngestResponse, UploadFileRequest,
};

/// Upload de arquivo ZIP para ingestão
///
/// Faz upload de um arquivo ZIP contendo arquivos COTAHIST (.TXT).
/// O sistema descompacta, parseia e converte os dados para Parquet.
/// A ingestão é incremental e idempotente.
#[utoipa::path(
    post,
    path = "/ingest/upload",
    tag = "ingest",
    request_body(
        content = UploadFileRequest,
        content_type = "multipart/form-data",
        description = "Arquivo ZIP do COTAHIST (.ZIP) para upload",
    ),
    responses(
        (status = 200, description = "Ingestão concluída com sucesso", body = IngestApiResponse),
        (status = 400, description = "Arquivo inválido ou muito grande", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno do servidor", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn upload_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> AppResult<Json<ApiResponse<IngestResponse>>> {
    let start = Instant::now();

    // Extract file from multipart
    let mut file_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::InvalidParam {
            message: format!("Erro ao ler upload: {}", e),
        })?
    {
        if field.name() == Some("file") {
            let data = field.bytes().await.map_err(|e| AppError::InvalidParam {
                message: format!("Erro ao ler dados do arquivo: {}", e),
            })?;
            file_data = Some(data.to_vec());
        }
    }

    let data = file_data.ok_or_else(|| AppError::InvalidParam {
        message: "Nenhum arquivo enviado. Use o campo 'file'.".to_string(),
    })?;

    // Validate file size (500 MB max)
    if data.len() > 500 * 1024 * 1024 {
        return Err(AppError::InvalidParam {
            message: "Arquivo muito grande (máximo 500 MB)".to_string(),
        });
    }

    // Run ingestion
    let data_dir = state.data_loader.data_config.dir.clone();
    let result = IngestService::ingest_zip(data, &data_dir, &state.data_loader)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    let message = if result.total_records_new > 0 {
        format!(
            "{} registros novos ingeridos ({} ignorados por já existirem)",
            result.total_records_new, result.total_records_skipped
        )
    } else {
        "Nenhum registro novo. Todos os dados já foram ingeridos anteriormente.".to_string()
    };

    Ok(Json(ApiResponse::new(
        IngestResponse {
            task_id: result.task_id,
            files_processed: result.files_processed,
            total_records_parsed: result.total_records_parsed,
            total_records_new: result.total_records_new,
            total_records_skipped: result.total_records_skipped,
            success: result.success,
            message,
        },
        elapsed,
    )))
}

/// Ingerir de pasta local
///
/// Ingerir arquivos COTAHIST de uma pasta local no servidor.
/// O caminho deve ser acessível pelo processo tucano-b3.
#[utoipa::path(
    post,
    path = "/ingest/folder",
    tag = "ingest",
    request_body = FolderIngestRequest,
    responses(
        (status = 200, description = "Ingestão concluída", body = IngestApiResponse),
        (status = 400, description = "Caminho inválido", body = ErrorResponse),
        (status = 404, description = "Pasta não encontrada", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn folder_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<FolderIngestRequest>,
) -> AppResult<Json<ApiResponse<IngestResponse>>> {
    let start = Instant::now();

    let folder = std::path::Path::new(&body.caminho);
    if !folder.exists() {
        return Err(AppError::InvalidParam {
            message: format!("Pasta não encontrada: {}", body.caminho),
        });
    }

    let data_dir = state.data_loader.data_config.dir.clone();
    let result = IngestService::ingest_folder(folder, &data_dir, &state.data_loader).map_err(
        |e| match e {
            crate::ingest::service::IngestError::Zip(
                crate::ingest::zip_handler::ZipError::NoCotahistFiles,
            ) => AppError::InvalidParam {
                message: format!(
                    "Nenhum arquivo COTAHIST (.TXT ou .ZIP) encontrado na pasta '{}'",
                    body.caminho
                ),
            },
            other => AppError::Internal(anyhow::anyhow!("{}", other)),
        },
    )?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;

    let message = if result.total_records_new > 0 {
        format!(
            "{} registros novos ingeridos ({} ignorados)",
            result.total_records_new, result.total_records_skipped
        )
    } else {
        "Nenhum registro novo.".to_string()
    };

    Ok(Json(ApiResponse::new(
        IngestResponse {
            task_id: result.task_id,
            files_processed: result.files_processed,
            total_records_parsed: result.total_records_parsed,
            total_records_new: result.total_records_new,
            total_records_skipped: result.total_records_skipped,
            success: result.success,
            message,
        },
        elapsed,
    )))
}

/// Ingerir ZIPs COTAHIST de um bucket S3 compatível
///
/// Lista os arquivos `.ZIP` cujo nome contém `COTAHIST` no bucket configurado
/// por `TUCANO_S3_*` e os ingere de forma incremental. Nada é baixado até esta
/// chamada. O corpo é opcional; `prefix` substitui `TUCANO_S3_PREFIX` nesta execução.
///
/// Compatível com Cloudflare R2 (`region=auto`, endpoint da conta, path-style).
/// Cada ZIP deve ter no máximo 500 MB. Pregões já registrados no ledger são ignorados.
#[utoipa::path(
    post,
    path = "/ingest/bucket",
    tag = "ingest",
    request_body = BucketIngestRequest,
    responses(
        (status = 200, description = "Ingestão concluída", body = IngestApiResponse),
        (status = 400, description = "Bucket não configurado, prefixo inválido, arquivo grande demais ou nenhum ZIP COTAHIST", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Erro interno", body = ErrorResponse),
    ),
    security(
        ("BearerAuth" = []),
        ("ApiKeyAuth" = []),
    ),
)]
pub(crate) async fn bucket_handler(
    State(state): State<Arc<AppState>>,
    body: Option<Json<BucketIngestRequest>>,
) -> AppResult<Json<ApiResponse<IngestResponse>>> {
    let start = Instant::now();

    let storage = S3ObjectStorage::from_config(&state.object_storage).map_err(|err| {
        AppError::InvalidParam {
            message: object_storage::object_storage_message(&err),
        }
    })?;

    let prefix = body
        .and_then(|Json(request)| request.prefix)
        .unwrap_or_else(|| state.object_storage.prefix.clone());

    let data_dir = state.data_loader.data_config.dir.clone();
    let result = storage
        .ingest_prefix(&prefix, &data_dir, &state.data_loader)
        .await
        .map_err(|err| AppError::InvalidParam {
            message: object_storage::object_storage_message(&err),
        })?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let message = if result.total_records_new > 0 {
        format!(
            "{} registros novos ingeridos a partir do bucket ({} ignorados)",
            result.total_records_new, result.total_records_skipped
        )
    } else {
        "Nenhum registro novo. Todos os dados do bucket já foram ingeridos anteriormente."
            .to_string()
    };

    Ok(Json(ApiResponse::new(
        IngestResponse {
            task_id: result.task_id,
            files_processed: result.files_processed,
            total_records_parsed: result.total_records_parsed,
            total_records_new: result.total_records_new,
            total_records_skipped: result.total_records_skipped,
            success: result.success,
            message,
        },
        elapsed,
    )))
}

/// Multipart framing on top of a ZIP at `MAX_ZIP_SIZE`.
const UPLOAD_BODY_LIMIT: usize = MAX_ZIP_SIZE + 1024 * 1024;

/// Annual COTAHIST archives take longer than the default API timeout to upload and parse.
pub const INGEST_REQUEST_TIMEOUT: Duration = Duration::from_secs(600);

/// Build ingest routes.
///
/// The upload route accepts bodies up to 500 MB plus multipart overhead. Axum's
/// default limit is 2 MB, which rejects COTAHIST ZIPs before the handler runs.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/ingest/upload",
            post(upload_handler).layer(DefaultBodyLimit::max(UPLOAD_BODY_LIMIT)),
        )
        .route("/ingest/folder", post(folder_handler))
        .route("/ingest/bucket", post(bucket_handler))
        .with_state(state)
}
