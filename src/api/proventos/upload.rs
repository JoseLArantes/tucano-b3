//! Multipart CSV upload for registered cash distributions.

use super::{map_proventos_error, require_configured_auth};
use crate::api::health::AppState;
#[allow(unused_imports)]
use crate::error::ErrorResponse;
use crate::error::{AppError, AppResult};
use crate::models::envelope::ApiResponse;
#[allow(unused_imports)]
use crate::models::envelope::ProventosCsvUploadApiResponse;
#[allow(unused_imports)]
use crate::models::proventos::ProventosCsvUploadRequest;
use crate::models::proventos::ProventosCsvUploadResponse;
use axum::{
    extract::{multipart::MultipartRejection, Multipart, State},
    http::StatusCode,
    Json,
};
use std::{sync::Arc, time::Instant};

const MAX_CSV_BYTES: usize = 1024 * 1024;
pub(super) const UPLOAD_BODY_LIMIT: usize = MAX_CSV_BYTES + 64 * 1024;

/// Importar proventos de um arquivo CSV
///
/// Envie um único arquivo no campo binário `file` do formulário multipart.
/// O CSV usa UTF-8 e separador vírgula, com cabeçalho obrigatório. Colunas
/// obrigatórias: `ticker`, `tipo` e `valor`. Colunas opcionais: `data_com`,
/// `data_pagamento` e `obs`. A ordem das colunas pode variar. Deixe células
/// opcionais vazias quando o dado estiver ausente.
///
/// Tipos aceitos: `Dividendo`, `JCP` e `Rendimento`. Use ponto decimal em
/// `valor` (reais por ação ou cota, até seis casas) e datas em `AAAA-MM-DD`.
/// Exemplo de CSV:
///
/// ```csv
/// ticker,tipo,valor,data_com,data_pagamento,obs
/// TAEE3,Dividendo,0.080300,2026-08-14,2026-11-26,Comunicado do emissor
/// ```
///
/// O limite é 1 MiB e 10.000 registros por arquivo. Toda linha é validada
/// antes da gravação. Um erro rejeita o lote inteiro. Registros que tenham
/// os mesmos seis campos após normalização são ignorados, inclusive dentro
/// do mesmo arquivo. A operação não atualiza registros existentes. A resposta
/// informa quantos registros foram recebidos, inseridos e ignorados.
#[utoipa::path(
    post,
    path = "/proventos/upload",
    tag = "proventos",
    request_body(
        content = ProventosCsvUploadRequest,
        content_type = "multipart/form-data",
        description = "Arquivo CSV UTF-8 no campo file"
    ),
    responses(
        (status = 200, description = "Lote validado e importado, com contagem de recebidos, inseridos e ignorados", body = ProventosCsvUploadApiResponse),
        (status = 400, description = "Formulário inválido, CSV vazio, cabeçalho ou registro inválido, ou mais de 10.000 registros", body = ErrorResponse),
        (status = 401, description = "Credencial ausente ou inválida", body = ErrorResponse),
        (status = 413, description = "Arquivo acima de 1 MiB ou formulário acima do limite multipart", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
        (status = 500, description = "Falha interna ao importar o lote no ledger", body = ErrorResponse),
        (status = 503, description = "Configure uma credencial antes de importar proventos", body = ErrorResponse),
    ),
    security(("BearerAuth" = []), ("ApiKeyAuth" = []))
)]
pub(crate) async fn upload_csv(
    State(state): State<Arc<AppState>>,
    multipart: Result<Multipart, MultipartRejection>,
) -> AppResult<Json<ApiResponse<ProventosCsvUploadResponse>>> {
    require_configured_auth(&state)?;
    let start = Instant::now();
    let mut multipart = multipart.map_err(|error| invalid_upload(error.to_string()))?;
    let mut file = None;
    while let Some(mut field) = multipart.next_field().await.map_err(multipart_error)? {
        if file.is_some() || field.name() != Some("file") || field.file_name().is_none() {
            return Err(invalid_upload("Envie um único arquivo no campo 'file'."));
        }
        let mut contents = Vec::new();
        while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
            if contents.len() + chunk.len() > MAX_CSV_BYTES {
                return Err(upload_too_large());
            }
            contents.extend_from_slice(&chunk);
        }
        file = Some(contents);
    }
    let file = file.ok_or_else(|| invalid_upload("Envie um arquivo CSV no campo 'file'."))?;

    let response = tokio::task::spawn_blocking(move || state.proventos.import_csv(&file))
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .map_err(map_proventos_error)?;
    Ok(Json(ApiResponse::new(
        response,
        start.elapsed().as_secs_f64() * 1000.0,
    )))
}

fn multipart_error(error: axum::extract::multipart::MultipartError) -> AppError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        upload_too_large()
    } else {
        invalid_upload(error.to_string())
    }
}

fn upload_too_large() -> AppError {
    AppError::PayloadTooLarge {
        message: "envie um CSV de até 1 MiB no campo 'file'".into(),
    }
}

fn invalid_upload(message: impl Into<String>) -> AppError {
    AppError::InvalidParam {
        message: message.into(),
    }
}
