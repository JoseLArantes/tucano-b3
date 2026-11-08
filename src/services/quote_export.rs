//! Revision-pinned, cached NDJSON exports for complete COTAHIST ticker histories.
use crate::services::query::{
    HistoricalQuote, QueryError, QueryPurpose, QueryService, NOMINAL_WARNING,
};
use axum::body::Bytes;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use flate2::{write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPool, Row};
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    sync::mpsc,
};
use tokio_stream::wrappers::ReceiverStream;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub const QUOTE_EXPORT_SCHEMA_VERSION: u16 = 1;
const BATCH_SIZE: i64 = 500;
const LEASE_SECONDS: i64 = 90;
const CACHE_DAYS: i64 = 30;
const TAIL_POLL: Duration = Duration::from_millis(200);
const MAX_TAIL_SECONDS: u64 = 60 * 60;

/// Filtros fechados para exportar todas as ocorrências de um ticker numa revisão COTAHIST.
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QuoteExportFilters {
    /// Código de negociação COTAHIST obrigatório; espaços externos são removidos e letras ASCII são normalizadas para maiúsculas. O ticker não é identidade permanente do instrumento.
    #[param(min_length = 1, max_length = 64, example = "PETR4")]
    pub ticker_symbol: String,
    /// UUID de revisão publicada. Se omitido sem cursor, a revisão ativa é escolhida uma vez no início; com cursor, usa-se a revisão contida nele. Se informado junto a um cursor, deve coincidir.
    #[param(example = "00000000-0000-4000-8000-000000000001")]
    pub data_revision: Option<Uuid>,
    /// Token opaco URL-safe sem padding, devolvido em `checkpoint`; retoma exclusivamente após a chave `(trade_date, id)` e permanece vinculado ao ticker, à revisão e à versão do schema.
    #[param(
        max_length = 1024,
        example = "eyJzY2hlbWFfdmVyc2lvbiI6MSwidGlja2VyX3N5bWJvbCI6IkVYTVAzIiwiZGF0YV9yZXZpc2lvbiI6IjAwMDAwMDAwLTAwMDAtNDAwMC04MDAwLTAwMDAwMDAwMDAwMDEiLCJ0cmFkZV9kYXRlIjoiMjAyNS0wMS0wMiIsInF1b3RlX2lkIjoiMDAwMDAwMDAtMDAwMC00MDAwLTgwMDAtMDAwMDAwMDAwMDEwIn0"
    )]
    pub cursor: Option<String>,
    /// Inclui `internal.version_id` e `internal.business_hash` em cada cotação; não concede autorização adicional.
    #[param(default = false, example = false)]
    pub include_internal: Option<bool>,
}

impl QuoteExportFilters {
    fn normalize(mut self) -> Result<Self, ExportError> {
        self.ticker_symbol = self.ticker_symbol.trim().to_ascii_uppercase();
        if self.ticker_symbol.is_empty() || self.ticker_symbol.chars().count() > 64 {
            return Err(ExportError::Invalid);
        }
        Ok(self)
    }
}

/// Linha individual do fluxo NDJSON versionado.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QuoteExportEvent {
    /// Metadados do conjunto imutável selecionado antes da primeira cotação.
    Meta {
        /// Versão do schema de eventos NDJSON.
        schema_version: u16,
        /// Ticker publicado filtrado, sem implicar identidade permanente.
        ticker_symbol: String,
        /// Revisão imutável que fixa o conjunto exportado.
        data_revision: Uuid,
        /// Cursor recebido, quando a resposta é uma retomada; `null` na primeira chamada.
        resumed_after: Option<String>,
        /// Aviso obrigatório de preços nominais sem ajuste retroativo.
        disclaimer: String,
    },
    /// Fato histórico oficial, com precisão, identidade e proveniência tipadas.
    Quote {
        /// Registro COTAHIST publicado pela B3.
        data: Box<HistoricalQuote>,
    },
    /// Ponto de retomada determinístico após um lote ordenado por data e UUID.
    Checkpoint {
        /// Cursor opaco a enviar novamente junto à mesma revisão e ticker.
        cursor: String,
        /// Quantidade acumulada de eventos `quote` nesta resposta até este checkpoint; uma retomada começa a contar novamente do zero.
        records_sent: u64,
    },
    /// Marcador de término verificável; seu checksum cobre as linhas anteriores.
    Complete {
        /// Quantidade de fatos nesta resposta, excluindo metadados/checkpoints.
        record_count: u64,
        /// Cursor final para continuar uma extração em chamadas posteriores.
        last_cursor: Option<String>,
        /// SHA-256 dos bytes UTF-8 das linhas NDJSON anteriores a este evento, incluindo cada LF final e excluindo a linha `complete`.
        content_sha256: String,
    },
    /// Erro depois do início do corpo; a ausência de `complete` também indica truncamento.
    Error {
        /// Código estável da falha de geração.
        code: String,
        /// Mensagem segura sem SQL, caminhos privados ou credenciais. O evento é emitido somente após o início da resposta HTTP.
        message: String,
        /// Identificador para correlacionar o erro no log da API.
        request_id: Uuid,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ExportError {
    #[error("invalid export filters")]
    Invalid,
    #[error("ticker not found")]
    NotFound,
    #[error("export service unavailable")]
    Unavailable,
    #[error("export storage failure")]
    Storage,
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Clone)]
pub(crate) struct QuoteExportService {
    pool: PgPool,
    root: PathBuf,
}

#[derive(Clone)]
pub(crate) struct PreparedQuoteExport {
    key: String,
    relative_path: String,
    ready: bool,
    raw_sha256: Option<String>,
    raw_byte_size: Option<u64>,
    gzip_sha256: Option<String>,
    gzip_byte_size: Option<u64>,
    lease_token: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportCursor {
    schema_version: u16,
    ticker_symbol: String,
    data_revision: Uuid,
    trade_date: chrono::NaiveDate,
    quote_id: Uuid,
}

#[derive(Serialize)]
struct ExportKeyMaterial<'a> {
    schema_version: u16,
    revision: Uuid,
    ticker: &'a str,
    include_internal: bool,
    after: Option<&'a ExportCursor>,
}

struct ExportBuild {
    query: QueryService,
    key: String,
    revision: Uuid,
    ticker: String,
    include_internal: bool,
    cursor: Option<ExportCursor>,
    lease_token: Uuid,
}

impl QuoteExportService {
    pub(crate) fn new(
        pool: PgPool,
        processed_root: impl Into<PathBuf>,
    ) -> Result<Self, ExportError> {
        let root = processed_root.into().join("quote-exports");
        std::fs::create_dir_all(&root).map_err(|_| ExportError::Storage)?;
        Ok(Self { pool, root })
    }

    /// Checks publication/elegibility before any cache lookup, pins the revision, and elects a builder.
    pub(crate) async fn prepare(
        &self,
        query: &QueryService,
        filters: QuoteExportFilters,
    ) -> Result<PreparedQuoteExport, ExportError> {
        let filters = filters.normalize()?;
        let ticker = filters.ticker_symbol.clone();
        let cursor = filters.cursor.as_deref().map(decode_cursor).transpose()?;
        if cursor.as_ref().is_some_and(|value| {
            value.ticker_symbol != ticker
                || filters
                    .data_revision
                    .is_some_and(|revision| revision != value.data_revision)
        }) {
            return Err(ExportError::Invalid);
        }
        let selected_revision = filters
            .data_revision
            .or_else(|| cursor.as_ref().map(|value| value.data_revision));
        let revision = query
            .resolve_revision(selected_revision, QueryPurpose::Rest)
            .await?;
        if cursor
            .as_ref()
            .is_some_and(|value| value.data_revision != revision)
        {
            return Err(ExportError::Invalid);
        }
        if !query.ticker_has_quotes(revision, &ticker).await? {
            return Err(ExportError::NotFound);
        }
        self.cleanup_expired().await?;
        let include_internal = filters.include_internal.unwrap_or(false);
        let key = export_key(revision, &ticker, include_internal, cursor.as_ref())?;
        let relative_path = format!("{key}.ndjson");

        if let Some(prepared) = self.ready_manifest(&key).await? {
            if self.raw_path(&prepared.relative_path).is_file() {
                return Ok(prepared);
            }
            self.mark_missing_artifact(&key).await?;
        }

        let lease_token = Uuid::new_v4();
        let claimed = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO b3_quote_export_artifacts(export_key,revision_id,ticker_symbol,schema_version,status,lease_token,lease_expires_at,relative_path,expires_at) VALUES($1,$2,$3,$4,'building',$5,NOW()+($6 * INTERVAL '1 second'),$7,NOW()+($8 * INTERVAL '1 day')) ON CONFLICT(export_key) DO UPDATE SET status='building',lease_token=EXCLUDED.lease_token,lease_expires_at=EXCLUDED.lease_expires_at,relative_path=EXCLUDED.relative_path,artifact_sha256=NULL,byte_size=NULL,gzip_sha256=NULL,gzip_byte_size=NULL,record_count=NULL,failure_code=NULL,updated_at=NOW(),expires_at=EXCLUDED.expires_at WHERE b3_quote_export_artifacts.status='failed' OR b3_quote_export_artifacts.expires_at<=NOW() OR (b3_quote_export_artifacts.status='building' AND b3_quote_export_artifacts.lease_expires_at<NOW()) RETURNING lease_token",
        )
        .bind(&key)
        .bind(revision)
        .bind(&ticker)
        .bind(i16::try_from(QUOTE_EXPORT_SCHEMA_VERSION).map_err(|_| ExportError::Unavailable)?)
        .bind(lease_token)
        .bind(LEASE_SECONDS)
        .bind(&relative_path)
        .bind(CACHE_DAYS)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(lease_token) = claimed {
            let service = self.clone();
            let builder_query = query.clone();
            let builder_ticker = ticker.clone();
            let builder_cursor = cursor.clone();
            let builder_key = key.clone();
            tokio::spawn(async move {
                service
                    .build(ExportBuild {
                        query: builder_query,
                        key: builder_key,
                        revision,
                        ticker: builder_ticker,
                        include_internal,
                        cursor: builder_cursor,
                        lease_token,
                    })
                    .await;
            });
            return Ok(PreparedQuoteExport {
                key,
                relative_path,
                ready: false,
                raw_sha256: None,
                raw_byte_size: None,
                gzip_sha256: None,
                gzip_byte_size: None,
                lease_token: Some(lease_token),
            });
        }

        if let Some(prepared) = self.ready_manifest(&key).await? {
            if self.raw_path(&prepared.relative_path).is_file() {
                return Ok(prepared);
            }
            self.mark_missing_artifact(&key).await?;
        }
        let row = sqlx::query("SELECT lease_token FROM b3_quote_export_artifacts WHERE export_key=$1 AND status='building'")
            .bind(&key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(ExportError::Unavailable)?;
        Ok(PreparedQuoteExport {
            key,
            relative_path,
            ready: false,
            raw_sha256: None,
            raw_byte_size: None,
            gzip_sha256: None,
            gzip_byte_size: None,
            lease_token: row.try_get("lease_token").ok(),
        })
    }

    async fn ready_manifest(&self, key: &str) -> Result<Option<PreparedQuoteExport>, ExportError> {
        let row = sqlx::query("SELECT revision_id,ticker_symbol,relative_path,artifact_sha256,byte_size,gzip_sha256,gzip_byte_size FROM b3_quote_export_artifacts WHERE export_key=$1 AND status='ready' AND expires_at>NOW()")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else { return Ok(None) };
        let relative_path: String = row.try_get("relative_path")?;
        if relative_path != format!("{key}.ndjson") {
            return Err(ExportError::Storage);
        }
        Ok(Some(PreparedQuoteExport {
            key: key.to_owned(),
            relative_path,
            ready: true,
            raw_sha256: row.try_get("artifact_sha256")?,
            raw_byte_size: row
                .try_get::<Option<i64>, _>("byte_size")?
                .map(|v| v as u64),
            gzip_sha256: row.try_get("gzip_sha256")?,
            gzip_byte_size: row
                .try_get::<Option<i64>, _>("gzip_byte_size")?
                .map(|v| v as u64),
            lease_token: None,
        }))
    }

    fn raw_path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    async fn mark_missing_artifact(&self, key: &str) -> Result<(), ExportError> {
        sqlx::query("UPDATE b3_quote_export_artifacts SET status='failed',failure_code='ARTIFACT_MISSING',updated_at=NOW() WHERE export_key=$1 AND status='ready'")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<(), ExportError> {
        static LAST_SWEEP: AtomicU64 = AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = LAST_SWEEP.load(Ordering::Relaxed);
        if now.saturating_sub(last) < 60
            || LAST_SWEEP
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_err()
        {
            return Ok(());
        }
        let rows = sqlx::query("WITH expired AS (SELECT export_key FROM b3_quote_export_artifacts WHERE status<>'building' AND expires_at<=NOW() ORDER BY expires_at LIMIT 50 FOR UPDATE SKIP LOCKED) DELETE FROM b3_quote_export_artifacts a USING expired e WHERE a.export_key=e.export_key RETURNING a.relative_path")
            .fetch_all(&self.pool)
            .await?;
        for row in rows {
            let relative: String = row.try_get("relative_path")?;
            if relative.len() == 72
                && relative.ends_with(".ndjson")
                && relative[..64].bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                let path = self.raw_path(&relative);
                let _ = fs::remove_file(&path).await;
                let _ = fs::remove_file(PathBuf::from(format!("{}.gz", path.display()))).await;
                let _ = fs::remove_file(PathBuf::from(format!("{}.part", path.display()))).await;
            }
        }
        Ok(())
    }

    pub(crate) fn ready_representation(
        &self,
        prepared: &PreparedQuoteExport,
        accept_gzip: bool,
    ) -> Option<(PathBuf, String, u64, bool)> {
        if !prepared.ready {
            return None;
        }
        let raw = self.raw_path(&prepared.relative_path);
        if accept_gzip {
            if let (Some(sha), Some(size)) = (&prepared.gzip_sha256, prepared.gzip_byte_size) {
                let path = self.root.join(format!("{}.gz", prepared.relative_path));
                if path.is_file() {
                    return Some((path, sha.clone(), size, true));
                }
            }
        }
        Some((
            raw,
            prepared.raw_sha256.clone()?,
            prepared.raw_byte_size?,
            false,
        ))
    }

    /// Streams complete cache entries or tails a cache build already being generated by another request.
    pub(crate) fn stream(
        &self,
        prepared: PreparedQuoteExport,
        representation: Option<(PathBuf, String, u64, bool)>,
    ) -> ReceiverStream<Result<Bytes, io::Error>> {
        let (sender, receiver) = mpsc::channel(8);
        let service = self.clone();
        tokio::spawn(async move {
            if let Some((path, _, expected_size, _)) = representation {
                service.copy_ready(path, expected_size, sender).await;
            } else {
                service.tail_build(prepared, sender).await;
            }
        });
        ReceiverStream::new(receiver)
    }

    async fn copy_ready(
        &self,
        path: PathBuf,
        expected_size: u64,
        sender: mpsc::Sender<Result<Bytes, io::Error>>,
    ) {
        let result = async {
            let mut file = File::open(path).await?;
            let mut remaining = expected_size;
            let mut buffer = vec![0; 64 * 1024];
            while remaining > 0 {
                let capacity = buffer.len().min(remaining as usize);
                let read = file.read(&mut buffer[..capacity]).await?;
                if read == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "export artifact truncated",
                    ));
                }
                remaining -= read as u64;
                if sender
                    .send(Ok(Bytes::copy_from_slice(&buffer[..read])))
                    .await
                    .is_err()
                {
                    return Ok(());
                }
            }
            Ok(())
        }
        .await;
        if let Err(error) = result {
            let _ = sender.send(Err(error)).await;
        }
    }

    async fn tail_build(
        &self,
        prepared: PreparedQuoteExport,
        sender: mpsc::Sender<Result<Bytes, io::Error>>,
    ) {
        let raw_path = self.raw_path(&prepared.relative_path);
        let started = tokio::time::Instant::now();
        let mut file: Option<File> = None;
        let mut observed_token = prepared.lease_token;
        let mut position = 0u64;
        let mut last_status_check = tokio::time::Instant::now() - TAIL_POLL;
        let mut buffer = vec![0; 64 * 1024];
        loop {
            if file.is_none() {
                let (status, token, expected_size, _) =
                    match self.manifest_state(&prepared.key).await {
                        Ok(state) => state,
                        Err(_) => {
                            let _ = sender
                                .send(Err(io::Error::other("export state unavailable")))
                                .await;
                            return;
                        }
                    };
                if let Some(first_token) = observed_token {
                    if token.is_some_and(|next| next != first_token) {
                        let _ = sender
                            .send(Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "export restarted; resume with the last checkpoint",
                            )))
                            .await;
                        return;
                    }
                } else {
                    observed_token = token;
                }
                let part_path = token.map(|token| {
                    self.root
                        .join(format!("{}.{}.ndjson.part", prepared.key, token))
                });
                let path = if status == "ready" {
                    Some(raw_path.as_path())
                } else {
                    part_path.as_deref()
                };
                let Some(path) = path else {
                    tokio::time::sleep(TAIL_POLL).await;
                    continue;
                };
                match File::open(path).await {
                    Ok(opened) => file = Some(opened),
                    Err(error) if error.kind() == io::ErrorKind::NotFound && status == "ready" => {
                        if expected_size == Some(0) {
                            return;
                        }
                        tokio::time::sleep(TAIL_POLL).await;
                        continue;
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        tokio::time::sleep(TAIL_POLL).await;
                        continue;
                    }
                    Err(error) => {
                        let _ = sender.send(Err(error)).await;
                        return;
                    }
                }
            }
            let current_file = file.as_mut().expect("file was opened");
            match current_file.read(&mut buffer).await {
                Ok(read) if read > 0 => {
                    position += read as u64;
                    if sender
                        .send(Ok(Bytes::copy_from_slice(&buffer[..read])))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
                Ok(_) => {
                    if started.elapsed().as_secs() > MAX_TAIL_SECONDS {
                        let _ = sender
                            .send(Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "export generation timed out",
                            )))
                            .await;
                        return;
                    }
                    if last_status_check.elapsed() >= Duration::from_secs(1) {
                        last_status_check = tokio::time::Instant::now();
                        match self.manifest_state(&prepared.key).await {
                            Ok((status, token, expected_size, lease_expires)) => {
                                if let Some(first_token) = observed_token {
                                    if token.is_some_and(|next| next != first_token) {
                                        let _ = sender
                                            .send(Err(io::Error::new(
                                                io::ErrorKind::Interrupted,
                                                "export restarted; resume with the last checkpoint",
                                            )))
                                            .await;
                                        return;
                                    }
                                }
                                if status == "ready"
                                    && expected_size.is_some_and(|size| position >= size)
                                {
                                    return;
                                }
                                if status == "failed"
                                    || (status == "building"
                                        && lease_expires
                                            .is_none_or(|expires| expires < chrono::Utc::now()))
                                {
                                    let _ = sender.send(Err(io::Error::new(io::ErrorKind::Interrupted, "export generation failed; retry with the last checkpoint"))).await;
                                    return;
                                }
                            }
                            Err(_) => {
                                let _ = sender
                                    .send(Err(io::Error::other("export state unavailable")))
                                    .await;
                                return;
                            }
                        }
                    }
                    tokio::time::sleep(TAIL_POLL).await;
                }
                Err(error) => {
                    let _ = sender.send(Err(error)).await;
                    return;
                }
            }
        }
    }

    async fn manifest_state(
        &self,
        key: &str,
    ) -> Result<
        (
            String,
            Option<Uuid>,
            Option<u64>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
        sqlx::Error,
    > {
        let row = sqlx::query("SELECT status,lease_token,byte_size,lease_expires_at FROM b3_quote_export_artifacts WHERE export_key=$1")
            .bind(key).fetch_one(&self.pool).await?;
        let size: Option<i64> = row.try_get("byte_size")?;
        Ok((
            row.try_get("status")?,
            row.try_get("lease_token")?,
            size.map(|value| value as u64),
            row.try_get("lease_expires_at")?,
        ))
    }

    async fn build(&self, build: ExportBuild) {
        let ExportBuild {
            query,
            key,
            revision,
            ticker,
            include_internal,
            cursor,
            lease_token,
        } = build;
        let relative = format!("{key}.ndjson");
        let final_path = self.raw_path(&relative);
        let part_path = self.root.join(format!("{key}.{lease_token}.ndjson.part"));
        let raw_file = match OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&part_path)
            .await
        {
            Ok(file) => file,
            Err(_) => {
                self.fail(&key, lease_token).await;
                return;
            }
        };
        let mut file = BufWriter::with_capacity(64 * 1024, raw_file);
        let mut hasher = Sha256::new();
        let mut count = 0u64;
        let mut last_cursor = cursor.clone();
        let resumed_after = cursor
            .as_ref()
            .map(encode_cursor)
            .transpose()
            .ok()
            .flatten();
        let meta = QuoteExportEvent::Meta {
            schema_version: QUOTE_EXPORT_SCHEMA_VERSION,
            ticker_symbol: ticker.clone(),
            data_revision: revision,
            resumed_after,
            disclaimer: NOMINAL_WARNING.to_owned(),
        };
        if write_event(&mut file, &mut hasher, &meta).await.is_err() {
            self.fail(&key, lease_token).await;
            return;
        }
        if file.flush().await.is_err() {
            self.fail(&key, lease_token).await;
            return;
        }

        loop {
            let after = last_cursor
                .as_ref()
                .map(|item| (item.trade_date, item.quote_id));
            let batch = match query
                .ticker_quote_batch(revision, &ticker, after, include_internal, BATCH_SIZE)
                .await
            {
                Ok(batch) => batch,
                Err(_) => {
                    let event = QuoteExportEvent::Error {
                        code: "EXPORT_FAILED".into(),
                        message: "A geração da exportação falhou; retome pelo último checkpoint."
                            .into(),
                        request_id: Uuid::new_v4(),
                    };
                    let _ = write_event(&mut file, &mut hasher, &event).await;
                    let _ = file.flush().await;
                    self.fail(&key, lease_token).await;
                    return;
                }
            };
            if batch.is_empty() {
                break;
            }
            for quote in &batch {
                let event = QuoteExportEvent::Quote {
                    data: Box::new(quote.clone()),
                };
                if write_event(&mut file, &mut hasher, &event).await.is_err() {
                    self.fail(&key, lease_token).await;
                    return;
                }
                count += 1;
                if (count == 1 || count.is_multiple_of(100)) && file.flush().await.is_err() {
                    self.fail(&key, lease_token).await;
                    return;
                }
            }
            if file.flush().await.is_err() {
                self.fail(&key, lease_token).await;
                return;
            }
            let final_quote = batch.last().expect("nonempty batch");
            let next_cursor = ExportCursor {
                schema_version: QUOTE_EXPORT_SCHEMA_VERSION,
                ticker_symbol: ticker.clone(),
                data_revision: revision,
                trade_date: final_quote.trade_date,
                quote_id: final_quote.id,
            };
            let encoded = match encode_cursor(&next_cursor) {
                Ok(value) => value,
                Err(_) => {
                    self.fail(&key, lease_token).await;
                    return;
                }
            };
            if write_event(
                &mut file,
                &mut hasher,
                &QuoteExportEvent::Checkpoint {
                    cursor: encoded,
                    records_sent: count,
                },
            )
            .await
            .is_err()
            {
                self.fail(&key, lease_token).await;
                return;
            }
            last_cursor = Some(next_cursor);
            if self.heartbeat(&key, lease_token).await.is_err() {
                return;
            }
        }

        let content_sha256 = hex::encode(hasher.clone().finalize());
        let complete = QuoteExportEvent::Complete {
            record_count: count,
            last_cursor: last_cursor
                .as_ref()
                .map(encode_cursor)
                .transpose()
                .ok()
                .flatten(),
            content_sha256,
        };
        if write_event(&mut file, &mut hasher, &complete)
            .await
            .is_err()
            || file.flush().await.is_err()
            || file.get_ref().sync_all().await.is_err()
        {
            self.fail(&key, lease_token).await;
            return;
        }
        drop(file);
        let artifact_sha256 = hex::encode(hasher.finalize());
        let byte_size = match fs::metadata(&part_path).await {
            Ok(metadata) => metadata.len() as i64,
            Err(_) => {
                self.fail(&key, lease_token).await;
                return;
            }
        };
        if fs::rename(&part_path, &final_path).await.is_err() {
            self.fail(&key, lease_token).await;
            return;
        }
        let updated = sqlx::query("UPDATE b3_quote_export_artifacts SET status='ready',lease_token=NULL,lease_expires_at=NULL,artifact_sha256=$3,byte_size=$4,record_count=$5,updated_at=NOW(),expires_at=NOW()+($6::text||' days')::interval WHERE export_key=$1 AND lease_token=$2 AND status='building'")
            .bind(&key).bind(lease_token).bind(&artifact_sha256).bind(byte_size).bind(count as i64).bind(CACHE_DAYS).execute(&self.pool).await;
        if updated.is_err() || updated.is_ok_and(|result| result.rows_affected() == 0) {
            return;
        }

        let gzip_path = self.root.join(format!("{relative}.gz"));
        let raw_for_compression = final_path.clone();
        let compress =
            tokio::task::spawn_blocking(move || compress_file(&raw_for_compression, &gzip_path));
        if let Ok(Ok((gzip_sha256, gzip_size))) = compress.await {
            let _ = sqlx::query("UPDATE b3_quote_export_artifacts SET gzip_sha256=$2,gzip_byte_size=$3,updated_at=NOW() WHERE export_key=$1 AND status='ready' AND artifact_sha256=$4")
                .bind(&key).bind(gzip_sha256).bind(gzip_size as i64).bind(artifact_sha256).execute(&self.pool).await;
        }
    }

    async fn heartbeat(&self, key: &str, lease_token: Uuid) -> Result<(), sqlx::Error> {
        let result = sqlx::query("UPDATE b3_quote_export_artifacts SET lease_expires_at=NOW()+($3 * INTERVAL '1 second'),updated_at=NOW() WHERE export_key=$1 AND lease_token=$2 AND status='building'")
            .bind(key).bind(lease_token).bind(LEASE_SECONDS).execute(&self.pool).await?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    }

    async fn fail(&self, key: &str, lease_token: Uuid) {
        let _ = sqlx::query("UPDATE b3_quote_export_artifacts SET status='failed',lease_token=NULL,lease_expires_at=NULL,failure_code='EXPORT_FAILED',updated_at=NOW() WHERE export_key=$1 AND lease_token=$2 AND status='building'")
            .bind(key).bind(lease_token).execute(&self.pool).await;
    }
}

fn export_key(
    revision: Uuid,
    ticker: &str,
    include_internal: bool,
    after: Option<&ExportCursor>,
) -> Result<String, ExportError> {
    let material = serde_json::to_vec(&ExportKeyMaterial {
        schema_version: QUOTE_EXPORT_SCHEMA_VERSION,
        revision,
        ticker,
        include_internal,
        after,
    })
    .map_err(|_| ExportError::Unavailable)?;
    Ok(hex::encode(Sha256::digest(material)))
}

fn encode_cursor(cursor: &ExportCursor) -> Result<String, ExportError> {
    let json = serde_json::to_vec(cursor).map_err(|_| ExportError::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(json))
}

fn decode_cursor(value: &str) -> Result<ExportCursor, ExportError> {
    if value.len() > 1024 {
        return Err(ExportError::Invalid);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| ExportError::Invalid)?;
    let cursor: ExportCursor = serde_json::from_slice(&bytes).map_err(|_| ExportError::Invalid)?;
    if cursor.schema_version != QUOTE_EXPORT_SCHEMA_VERSION {
        return Err(ExportError::Invalid);
    }
    Ok(cursor)
}

async fn write_event<W>(
    file: &mut W,
    hasher: &mut Sha256,
    event: &QuoteExportEvent,
) -> io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut line = serde_json::to_vec(event).map_err(io::Error::other)?;
    line.push(b'\n');
    file.write_all(&line).await?;
    hasher.update(line);
    Ok(())
}

fn compress_file(raw: &Path, compressed: &Path) -> io::Result<(String, u64)> {
    let input = std::fs::File::open(raw)?;
    let temp = PathBuf::from(format!("{}.tmp", compressed.display()));
    let output = std::fs::File::create(&temp)?;
    let mut encoder = GzEncoder::new(output, Compression::fast());
    let mut reader = std::io::BufReader::new(input);
    io::copy(&mut reader, &mut encoder)?;
    let output = encoder.finish()?;
    output.sync_all()?;
    drop(output);
    std::fs::rename(&temp, compressed)?;
    let mut file = std::fs::File::open(compressed)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((hex::encode(hasher.finalize()), size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn cursor_round_trips_and_is_bound_to_schema_ticker_and_revision() {
        let cursor = ExportCursor {
            schema_version: QUOTE_EXPORT_SCHEMA_VERSION,
            ticker_symbol: "PETR4".into(),
            data_revision: Uuid::new_v4(),
            trade_date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            quote_id: Uuid::new_v4(),
        };
        let encoded = encode_cursor(&cursor).unwrap();
        assert_eq!(decode_cursor(&encoded).unwrap().quote_id, cursor.quote_id);
        assert!(decode_cursor("not-a-cursor").is_err());
    }

    #[test]
    fn export_key_changes_for_revision_and_internal_projection() {
        let revision = Uuid::new_v4();
        let public = export_key(revision, "PETR4", false, None).unwrap();
        assert_ne!(public, export_key(revision, "PETR4", true, None).unwrap());
        assert_ne!(
            public,
            export_key(Uuid::new_v4(), "PETR4", false, None).unwrap()
        );
    }

    #[test]
    fn gzip_artifact_round_trips_and_reports_representation_digest() {
        use flate2::read::GzDecoder;

        let directory = tempfile::tempdir().unwrap();
        let raw_path = directory.path().join("source.ndjson");
        let gzip_path = directory.path().join("source.ndjson.gz");
        let content = b"{\"type\":\"meta\"}\n{\"type\":\"complete\"}\n";
        std::fs::write(&raw_path, content).unwrap();

        let (digest, byte_size) = compress_file(&raw_path, &gzip_path).unwrap();
        let compressed = std::fs::read(&gzip_path).unwrap();
        let mut decoder = GzDecoder::new(compressed.as_slice());
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).unwrap();

        assert_eq!(decoded, content);
        assert_eq!(byte_size as usize, compressed.len());
        assert_eq!(digest, hex::encode(Sha256::digest(compressed)));
    }
}
