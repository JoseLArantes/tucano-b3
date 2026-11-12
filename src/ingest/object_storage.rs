//! On-demand ingestion of COTAHIST ZIP archives from S3-compatible storage.
//!
//! The client is built from `TUCANO_S3_*` environment variables. Nothing is
//! downloaded until `POST /api/v1/ingest/bucket` runs.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use rusty_s3::actions::{GetObject, ListObjectsV2, S3Action};
use rusty_s3::{Bucket, Credentials, UrlStyle};
use thiserror::Error;

use crate::config::ObjectStorageConfig;
use crate::data::reload::DataLoader;
use crate::ingest::service::{IngestError, IngestResult, IngestService};
use crate::ingest::zip_handler::{ZipError, MAX_ZIP_SIZE};

/// Stop listing if a prefix matches more keys than an operator would place by hand.
const MAX_KEYS_SCANNED: usize = 10_000;
const SIGNED_URL_TTL: Duration = Duration::from_secs(300);

/// One ZIP object selected for ingestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedObject {
    pub key: String,
    pub size: u64,
}

/// Failures while reading or ingesting a bucket.
#[derive(Debug, Error)]
pub enum ObjectStorageError {
    #[error("not configured")]
    NotConfigured,

    #[error("invalid prefix")]
    InvalidPrefix(String),

    #[error("no COTAHIST zip files")]
    NoCotahistZips,

    #[error("object too large")]
    TooLarge { key: String, size: u64, max: u64 },

    #[error("{message}")]
    Storage { message: String },

    #[error("ingest failed for {key}")]
    Ingest { key: String, source: IngestError },
}

/// True when the object key is a COTAHIST `.ZIP` archive.
///
/// Matching uses the file name, so `cotahist/COTAHIST_A2025.ZIP` qualifies and
/// `notes/readme.txt` does not.
pub fn is_cotahist_zip_key(key: &str) -> bool {
    let name = key.rsplit(['/', '\\']).next().unwrap_or(key);
    if name.is_empty() || name.ends_with('/') {
        return false;
    }
    let upper = name.to_ascii_uppercase();
    upper.contains("COTAHIST") && upper.ends_with(".ZIP")
}

/// Normalize an S3 key prefix. Empty means the bucket root.
pub fn normalize_prefix(prefix: &str) -> Result<String, ObjectStorageError> {
    let trimmed = prefix.trim().trim_start_matches('/');
    if trimmed.contains("..") || trimmed.contains('\\') {
        return Err(ObjectStorageError::InvalidPrefix(
            "Prefixo inválido. Use um prefixo de chave S3, como 'cotahist/'.".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// S3-compatible client (AWS S3, Cloudflare R2, MinIO).
pub struct S3ObjectStorage {
    http: reqwest::Client,
    bucket: Bucket,
    credentials: Credentials,
    bucket_name: String,
    secret_access_key: String,
}

impl S3ObjectStorage {
    /// Build a client. Does not connect until a request is sent.
    pub fn from_config(config: &ObjectStorageConfig) -> Result<Self, ObjectStorageError> {
        if !config.is_configured() {
            return Err(ObjectStorageError::NotConfigured);
        }

        let endpoint = config.endpoint.trim().trim_end_matches('/');
        if !(endpoint.starts_with("https://") || endpoint.starts_with("http://")) {
            return Err(ObjectStorageError::Storage {
                message: "TUCANO_S3_ENDPOINT deve ser uma URL http:// ou https://.".to_string(),
            });
        }

        let endpoint_url = endpoint.parse().map_err(|_| ObjectStorageError::Storage {
            message: "TUCANO_S3_ENDPOINT não é uma URL válida.".to_string(),
        })?;
        let style = if config.force_path_style {
            UrlStyle::Path
        } else {
            UrlStyle::VirtualHost
        };
        let bucket_name = config.bucket.trim().to_string();
        let region = config.region().to_string();
        let bucket = Bucket::new(
            endpoint_url,
            style,
            bucket_name.clone(),
            region,
        )
        .map_err(|_| ObjectStorageError::Storage {
            message: "Não foi possível montar o endpoint do bucket. Confira TUCANO_S3_ENDPOINT e TUCANO_S3_BUCKET.".to_string(),
        })?;

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .map_err(|err| ObjectStorageError::Storage {
                message: format!("Falha ao criar o cliente HTTP: {}", err.without_url()),
            })?;

        Ok(Self {
            http,
            bucket,
            credentials: Credentials::new(
                config.access_key_id.trim().to_string(),
                config.secret_access_key.trim().to_string(),
            ),
            bucket_name,
            secret_access_key: config.secret_access_key.clone(),
        })
    }

    /// List COTAHIST ZIP keys under `prefix`, in deterministic order.
    pub async fn list_cotahist_zips(
        &self,
        prefix: &str,
    ) -> Result<Vec<ListedObject>, ObjectStorageError> {
        let prefix = normalize_prefix(prefix)?;
        let mut continuation: Option<String> = None;
        let mut scanned = 0usize;
        let mut selected = Vec::new();

        loop {
            let mut action = ListObjectsV2::new(&self.bucket, Some(&self.credentials));
            action.with_max_keys(1000);
            if !prefix.is_empty() {
                action.with_prefix(prefix.as_str());
            }
            if let Some(token) = continuation.as_deref() {
                action.with_continuation_token(token);
            }

            let body = self
                .signed_get(action.sign(SIGNED_URL_TTL), "listar o bucket")
                .await?;
            let page = ListObjectsV2::parse_response(&body).map_err(|err| {
                ObjectStorageError::Storage {
                    message: format!("Falha ao ler a listagem do bucket: {err}"),
                }
            })?;

            for object in page.contents {
                scanned += 1;
                if scanned > MAX_KEYS_SCANNED {
                    return Err(ObjectStorageError::Storage {
                        message: "A listagem excedeu 10000 chaves. Defina um prefixo mais específico em TUCANO_S3_PREFIX ou no corpo da requisição.".to_string(),
                    });
                }
                let key = decode_s3_key(&object.key);
                if !is_cotahist_zip_key(&key) {
                    continue;
                }
                selected.push(ListedObject {
                    key,
                    size: object.size,
                });
            }

            match page.next_continuation_token {
                Some(token) if !token.is_empty() => continuation = Some(token),
                _ => break,
            }
        }

        selected.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(selected)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, ObjectStorageError> {
        let action = GetObject::new(&self.bucket, Some(&self.credentials), key);
        self.signed_get(action.sign(SIGNED_URL_TTL), &format!("baixar '{key}'"))
            .await
    }

    async fn signed_get(&self, url: url::Url, action: &str) -> Result<Vec<u8>, ObjectStorageError> {
        let response =
            self.http
                .get(url)
                .send()
                .await
                .map_err(|err| ObjectStorageError::Storage {
                    message: format!(
                        "Falha ao {action}: {}",
                        redact(&err.without_url().to_string(), &self.secret_access_key)
                    ),
                })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(300).collect();
            return Err(ObjectStorageError::Storage {
                message: redact(
                    &format!("Falha ao {action} (HTTP {status}): {snippet}"),
                    &self.secret_access_key,
                ),
            });
        }

        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|err| ObjectStorageError::Storage {
                message: format!(
                    "Falha ao ler a resposta ao {action}: {}",
                    redact(&err.without_url().to_string(), &self.secret_access_key)
                ),
            })
    }

    /// Download every COTAHIST ZIP under `prefix` and ingest it.
    ///
    /// Archives are processed one at a time. A later failure leaves earlier
    /// trading dates committed; repeating the call is safe.
    pub async fn ingest_prefix(
        &self,
        prefix: &str,
        data_dir: &Path,
        data_loader: &Arc<DataLoader>,
    ) -> Result<IngestResult, ObjectStorageError> {
        let objects = self.list_cotahist_zips(prefix).await?;
        if objects.is_empty() {
            return Err(ObjectStorageError::NoCotahistZips);
        }

        let max = MAX_ZIP_SIZE as u64;
        for object in &objects {
            if object.size > max {
                return Err(ObjectStorageError::TooLarge {
                    key: object.key.clone(),
                    size: object.size,
                    max,
                });
            }
        }

        let task_id = uuid::Uuid::new_v4().to_string();
        let mut combined: Option<IngestResult> = None;

        for object in objects {
            tracing::info!(
                task_id = %task_id,
                bucket = %self.bucket_name,
                key = %object.key,
                size = object.size,
                "Baixando ZIP COTAHIST do bucket"
            );
            let bytes = self.download(&object.key).await?;
            if bytes.len() as u64 > max {
                return Err(ObjectStorageError::TooLarge {
                    key: object.key,
                    size: bytes.len() as u64,
                    max,
                });
            }

            let result =
                IngestService::ingest_zip(&bytes, data_dir, data_loader).map_err(|source| {
                    ObjectStorageError::Ingest {
                        key: object.key.clone(),
                        source,
                    }
                })?;

            combined = Some(match combined {
                None => {
                    let mut result = result;
                    result.task_id = task_id.clone();
                    result
                }
                Some(mut acc) => {
                    acc.files_processed += result.files_processed;
                    acc.total_records_parsed += result.total_records_parsed;
                    acc.total_records_new += result.total_records_new;
                    acc.total_records_skipped += result.total_records_skipped;
                    acc.file_results.extend(result.file_results);
                    acc.success = acc.success && result.success;
                    acc
                }
            });
        }

        combined.ok_or(ObjectStorageError::NoCotahistZips)
    }
}

/// `ListObjectsV2` is requested with `encoding-type=url`.
fn decode_s3_key(key: &str) -> String {
    percent_encoding::percent_decode_str(key)
        .decode_utf8_lossy()
        .into_owned()
}

fn redact(message: &str, secret: &str) -> String {
    if secret.is_empty() {
        message.to_string()
    } else {
        message.replace(secret, "<redacted>")
    }
}

/// Map a storage failure to the Portuguese message returned by the API.
pub fn object_storage_message(err: &ObjectStorageError) -> String {
    match err {
        ObjectStorageError::NotConfigured => {
            "Armazenamento de objetos não configurado. Defina TUCANO_S3_ENDPOINT, TUCANO_S3_BUCKET, TUCANO_S3_ACCESS_KEY_ID e TUCANO_S3_SECRET_ACCESS_KEY.".to_string()
        }
        ObjectStorageError::InvalidPrefix(message) => message.clone(),
        ObjectStorageError::NoCotahistZips => {
            "Nenhum arquivo COTAHIST (.ZIP) encontrado no bucket.".to_string()
        }
        ObjectStorageError::TooLarge { key, size, max } => format!(
            "Arquivo '{key}' excede o limite de 500 MB ({size} bytes, máximo {max})."
        ),
        ObjectStorageError::Storage { message } => message.clone(),
        ObjectStorageError::Ingest { key, source } => match source {
            IngestError::Zip(ZipError::NoCotahistFiles) => {
                format!("O arquivo '{key}' não contém COTAHIST (.TXT).")
            }
            other => format!("Falha ao ingerir '{key}': {other}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cotahist_zip_keys_match_file_name_only() {
        assert!(is_cotahist_zip_key("COTAHIST_A2025.ZIP"));
        assert!(is_cotahist_zip_key("cotahist/COTAHIST_A2025.zip"));
        assert!(is_cotahist_zip_key("incoming/cotahist_d20260102.ZIP"));
        assert!(!is_cotahist_zip_key("cotahist/readme.txt"));
        assert!(!is_cotahist_zip_key("COTAHIST_A2025.TXT"));
        assert!(!is_cotahist_zip_key("other/archive.ZIP"));
        assert!(!is_cotahist_zip_key("cotahist/"));
    }

    #[test]
    fn prefix_rejects_parent_segments() {
        assert_eq!(normalize_prefix(" /cotahist/ ").unwrap(), "cotahist/");
        assert!(normalize_prefix("../secret").is_err());
        assert!(normalize_prefix("cotahist\\a").is_err());
    }

    #[test]
    fn encoded_object_keys_are_decoded() {
        assert_eq!(
            decode_s3_key("cotahist/COTAHIST_A2025.ZIP"),
            "cotahist/COTAHIST_A2025.ZIP"
        );
        assert_eq!(
            decode_s3_key("cotahist/COTAHIST%20A2025.ZIP"),
            "cotahist/COTAHIST A2025.ZIP"
        );
    }

    #[test]
    fn storage_errors_hide_the_secret() {
        let err = ObjectStorageError::Storage {
            message: redact("signature secret-value failed", "secret-value"),
        };
        let message = object_storage_message(&err);
        assert!(!message.contains("secret-value"));
        assert!(message.contains("<redacted>"));
    }
}
