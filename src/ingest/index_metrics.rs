//! Acquisition, parsing, persistence, and scheduling for B3 index evolution CSVs.

use crate::data::reload::DataLoader;
use crate::data::schema::index_metrics_schema;
use arrow::array::{Date32Array, Int64Array, StringArray, TimestampNanosecondArray};
use arrow::record_batch::RecordBatch;
use base64::Engine;
use chrono::{Datelike, NaiveDate, Timelike, Utc};
use chrono_tz::America::Sao_Paulo;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex as AsyncMutex;
use utoipa::ToSchema;

const SOURCE: &str = "https://sistemaswebb3-listados.b3.com.br/indexStatisticsProxy/IndexCall/GetDownloadPortfolioDay/";
const MONTHS: [&str; 12] = [
    "Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum IndexCode {
    Ibov,
    Idiv,
    Ifix,
}

impl IndexCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ibov => "IBOV",
            Self::Idiv => "IDIV",
            Self::Ifix => "IFIX",
        }
    }
    fn b3_code(self) -> &'static str {
        match self {
            Self::Ibov => "IBOVESPA",
            Self::Idiv => "IDIV",
            Self::Ifix => "IFIX",
        }
    }
    fn first_year(self) -> i32 {
        match self {
            Self::Ibov => 1998,
            Self::Idiv => 2005,
            Self::Ifix => 2010,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexPoint {
    pub date: NaiveDate,
    pub close_cents: i64,
}

#[derive(Debug, Error)]
pub enum IndexIngestError {
    #[error("HTTP source request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Malformed B3 CSV: {0}")]
    Csv(String),
    #[error("Persistence failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite ledger failed: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("Base SQLite ledger failed: {0}")]
    Ledger(#[from] crate::ledger::LedgerError),
    #[error("Parquet write failed: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),
    #[error("Data reload failed: {0}")]
    Reload(String),
    #[error("No daily values found for {index} {year}")]
    Empty { index: String, year: i32 },
}

pub fn parse_b3_csv(
    index: IndexCode,
    year: i32,
    bytes: &[u8],
) -> Result<Vec<IndexPoint>, IndexIngestError> {
    if bytes.len() > 2_000_000 {
        return Err(IndexIngestError::Csv("download excede 2 MB".into()));
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(bytes.trim_ascii())
        .map_err(|e| IndexIngestError::Csv(format!("Base64 inválido: {e}")))?;
    let text: String = decoded.iter().map(|b| char::from(*b)).collect();
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let rows = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| IndexIngestError::Csv(e.to_string()))?;
    if rows.len() < 3
        || !rows[0]
            .get(0)
            .unwrap_or_default()
            .starts_with(&format!("{} - {year}", index.b3_code()))
    {
        return Err(IndexIngestError::Csv(format!(
            "identificação do índice/ano não confere: {:?}",
            rows.first().and_then(|row| row.get(0))
        )));
    }
    for (i, month) in MONTHS.iter().enumerate() {
        if rows[1].get(i + 1) != Some(*month) {
            return Err(IndexIngestError::Csv("cabeçalho mensal inesperado".into()));
        }
    }
    let mut points = BTreeMap::new();
    let mut days = std::collections::HashSet::new();
    for row in rows.iter().skip(2) {
        let day_text = row.get(0).unwrap_or_default().trim();
        if day_text.is_empty()
            || day_text.starts_with("MÍNIMO")
            || day_text.starts_with("MÁXIMO")
            || day_text.starts_with("MINIMO")
            || day_text.starts_with("MAXIMO")
        {
            continue;
        }
        let day: u32 = day_text
            .parse()
            .map_err(|_| IndexIngestError::Csv(format!("dia inválido: {day_text}")))?;
        if !(1..=31).contains(&day) || !days.insert(day) {
            return Err(IndexIngestError::Csv(format!(
                "dia inválido ou duplicado: {day_text}"
            )));
        }
        if row.len() != 13 {
            return Err(IndexIngestError::Csv(
                "linha com número inválido de colunas".into(),
            ));
        }
        for month in 1..=12 {
            let value = row.get(month).unwrap_or_default().trim();
            if value.is_empty() {
                continue;
            }
            let date = NaiveDate::from_ymd_opt(year, month as u32, day).ok_or_else(|| {
                IndexIngestError::Csv(format!("data inválida {year}-{month}-{day}"))
            })?;
            let close_cents = parse_brazilian_decimal(value)?;
            if points.insert(date, close_cents).is_some() {
                return Err(IndexIngestError::Csv(format!("data duplicada {date}")));
            }
        }
    }
    if days.len() != 31 {
        return Err(IndexIngestError::Csv(format!(
            "arquivo anual incompleto: {} linhas diárias, esperado 31",
            days.len()
        )));
    }
    Ok(points
        .into_iter()
        .map(|(date, close_cents)| IndexPoint { date, close_cents })
        .collect())
}

fn schedule_due(now: chrono::DateTime<chrono_tz::Tz>, completed: bool) -> bool {
    now.with_timezone(&Sao_Paulo).hour() >= 2 && !completed
}

fn scheduled_years(now: chrono::DateTime<chrono_tz::Tz>) -> Vec<i32> {
    let local = now.with_timezone(&Sao_Paulo);
    if local.month() == 1 && local.day() == 1 {
        vec![local.year() - 1, local.year()]
    } else {
        vec![local.year()]
    }
}

fn parse_brazilian_decimal(value: &str) -> Result<i64, IndexIngestError> {
    let (whole, fraction) = value
        .split_once(',')
        .ok_or_else(|| IndexIngestError::Csv(format!("decimal inválido: {value}")))?;
    if fraction.len() != 2 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return Err(IndexIngestError::Csv(format!("precisão inválida: {value}")));
    }
    let whole = whole.replace('.', "");
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
        return Err(IndexIngestError::Csv(format!("decimal inválido: {value}")));
    }
    let units: i64 = whole
        .parse()
        .map_err(|_| IndexIngestError::Csv(format!("valor fora do limite: {value}")))?;
    let cents: i64 = fraction.parse().unwrap_or(0);
    units
        .checked_mul(100)
        .and_then(|n| n.checked_add(cents))
        .ok_or_else(|| IndexIngestError::Csv(format!("valor fora do limite: {value}")))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ManualIngestRequest {
    /// Índices a atualizar. Ausência seleciona IBOV, IDIV e IFIX.
    pub indices: Option<Vec<IndexCode>>,
    /// Primeiro ano inclusivo. Ausência seleciona o ano atual.
    pub ano_inicio: Option<i32>,
    /// Último ano inclusivo. Ausência seleciona o ano atual.
    pub ano_fim: Option<i32>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct IngestJob {
    /// Identificador persistido da execução.
    pub id: String,
    /// Estado queued, running, succeeded ou failed.
    pub status: String,
    /// Horário de criação em UTC.
    pub created_at: String,
    /// Resultado individual para cada índice e ano.
    pub results: Vec<String>,
}

pub struct IndexIngestor {
    data_dir: PathBuf,
    loader: Arc<DataLoader>,
    lock: AsyncMutex<()>,
    db: Mutex<Connection>,
}

impl IndexIngestor {
    pub fn new(data_dir: &Path, loader: Arc<DataLoader>) -> Result<Arc<Self>, IndexIngestError> {
        let path = data_dir.join("ledger/tucano.db");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        drop(crate::ledger::Ledger::open(&path)?);
        let db = Connection::open(path)?;
        db.execute_batch("CREATE TABLE IF NOT EXISTS index_ingest_jobs (id TEXT PRIMARY KEY, status TEXT NOT NULL, created_at TEXT NOT NULL, results TEXT NOT NULL DEFAULT '[]', request_json TEXT NOT NULL DEFAULT '{}'); CREATE TABLE IF NOT EXISTS index_ingest_years (index_code TEXT NOT NULL, year INTEGER NOT NULL, source_hash TEXT NOT NULL, row_count INTEGER NOT NULL, completed_at TEXT NOT NULL, PRIMARY KEY(index_code, year)); CREATE TABLE IF NOT EXISTS index_partitions (index_code TEXT NOT NULL, year INTEGER NOT NULL, path TEXT NOT NULL, source_hash TEXT NOT NULL, row_count INTEGER NOT NULL, completed_at TEXT NOT NULL, PRIMARY KEY(index_code,year)); CREATE TABLE IF NOT EXISTS index_ingest_schedule (local_date TEXT PRIMARY KEY, completed_at TEXT NOT NULL);")?;
        let has_request = db
            .prepare("PRAGMA table_info(index_ingest_jobs)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(Result::ok)
            .any(|column| column == "request_json");
        if !has_request {
            db.execute(
                "ALTER TABLE index_ingest_jobs ADD COLUMN request_json TEXT NOT NULL DEFAULT '{}'",
                [],
            )?;
        }
        db.execute("UPDATE index_ingest_jobs SET status='failed',results='[\"Interrompida pela reinicialização\"]' WHERE status IN ('queued','running') AND request_json='{}'",[])?;
        Ok(Arc::new(Self {
            data_dir: data_dir.to_path_buf(),
            loader,
            lock: AsyncMutex::new(()),
            db: Mutex::new(db),
        }))
    }

    pub fn create_job(
        self: &Arc<Self>,
        request: ManualIngestRequest,
    ) -> Result<String, IndexIngestError> {
        let current = chrono::Utc::now().with_timezone(&Sao_Paulo).year();
        let start = request.ano_inicio.unwrap_or(current);
        let end = request.ano_fim.unwrap_or(current);
        if start < 1998
            || end > current
            || start > end
            || request.indices.as_ref().is_some_and(|v| v.is_empty())
        {
            return Err(IndexIngestError::Csv(
                "intervalo ou seleção de índices inválido".into(),
            ));
        }
        let indices = request
            .indices
            .unwrap_or_else(|| vec![IndexCode::Ibov, IndexCode::Idiv, IndexCode::Ifix]);
        if indices.iter().any(|index| start < index.first_year()) {
            return Err(IndexIngestError::Csv(
                "ano inicial anterior à cobertura do índice selecionado".into(),
            ));
        }
        let request_json = serde_json::to_string(&ManualIngestRequest {
            indices: Some(indices.clone()),
            ano_inicio: Some(start),
            ano_fim: Some(end),
        })
        .unwrap_or_else(|_| "{}".into());
        let id = format!(
            "idx-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        let created = Utc::now().to_rfc3339();
        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM index_ingest_jobs WHERE status IN ('queued','running'))",
            [],
            |row| row.get(0),
        )?;
        if active {
            return Err(IndexIngestError::Csv(
                "an index ingestion is already active".into(),
            ));
        }
        tx.execute("INSERT INTO index_ingest_jobs(id,status,created_at,request_json) VALUES(?1,'queued',?2,?3)", params![id,created,request_json])?;
        tx.commit()?;
        drop(db);
        let ingestor = Arc::clone(self);
        let job = id.clone();
        tokio::spawn(async move {
            ingestor.run_job(job, indices, start, end, "manual").await;
        });
        Ok(id)
    }

    pub fn job(&self, id: &str) -> Result<Option<IngestJob>, IndexIngestError> {
        let db = self.db.lock().unwrap();
        let mut statement =
            db.prepare("SELECT id,status,created_at,results FROM index_ingest_jobs WHERE id=?1")?;
        let mut rows = statement.query([id])?;
        if let Some(row) = rows.next()? {
            let results: String = row.get(3)?;
            Ok(Some(IngestJob {
                id: row.get(0)?,
                status: row.get(1)?,
                created_at: row.get(2)?,
                results: serde_json::from_str(&results).unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn has_active_job(&self) -> Result<bool, IndexIngestError> {
        let db = self.db.lock().unwrap();
        Ok(db.query_row(
            "SELECT EXISTS(SELECT 1 FROM index_ingest_jobs WHERE status IN ('queued','running'))",
            [],
            |row| row.get::<_, bool>(0),
        )?)
    }

    pub fn last_successful_ingestion(&self) -> Result<Option<String>, IndexIngestError> {
        let db = self.db.lock().unwrap();
        Ok(db.query_row(
            "SELECT MAX(completed_at) FROM index_partitions",
            [],
            |row| row.get(0),
        )?)
    }

    fn interrupted_jobs(&self) -> Result<Vec<(String, ManualIngestRequest)>, IndexIngestError> {
        let db = self.db.lock().unwrap();
        let mut statement = db.prepare(
            "SELECT id,request_json FROM index_ingest_jobs WHERE status IN ('queued','running') AND request_json!='{}'",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut jobs = Vec::new();
        for row in rows {
            let (id, request) = row?;
            if let Ok(request) = serde_json::from_str(&request) {
                jobs.push((id, request));
            }
        }
        Ok(jobs)
    }

    async fn run_job(
        self: Arc<Self>,
        id: String,
        indices: Vec<IndexCode>,
        start: i32,
        end: i32,
        origin: &'static str,
    ) {
        let _guard = self.lock.lock().await;
        let _ = self.update_job(&id, "running", Vec::new());
        let mut results = Vec::new();
        let mut success = true;
        for index in indices {
            for year in start..=end {
                tracing::info!(job_id=%id, origin, index=index.as_str(), year, "Starting B3 index CSV ingestion");
                match self.ingest_year(index, year).await {
                    Ok(count) => results.push(format!("{}:{year}:ok:{count}", index.as_str())),
                    Err(error) => {
                        success = false;
                        results.push(format!("{}:{year}:error:{error}", index.as_str()));
                        tracing::error!(job_id=%id, origin, index=index.as_str(), year, %error, "B3 index CSV ingestion failed");
                    }
                }
                let _ = self.update_job(&id, "running", results.clone());
            }
        }
        let _ = self.update_job(&id, if success { "succeeded" } else { "failed" }, results);
    }

    fn update_job(
        &self,
        id: &str,
        status: &str,
        results: Vec<String>,
    ) -> Result<(), IndexIngestError> {
        self.db.lock().unwrap().execute(
            "UPDATE index_ingest_jobs SET status=?2,results=?3 WHERE id=?1",
            params![
                id,
                status,
                serde_json::to_string(&results).unwrap_or_default()
            ],
        )?;
        Ok(())
    }

    async fn ingest_year(&self, index: IndexCode, year: i32) -> Result<usize, IndexIngestError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        let request = serde_json::json!({"index": index.b3_code(), "language":"pt-br", "year":year.to_string()});
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_vec(&request).unwrap_or_default());
        let mut payload = None;
        for attempt in 0..=2 {
            match client.get(format!("{SOURCE}{encoded}")).send().await {
                Ok(mut response) if response.status().is_success() => {
                    if response
                        .content_length()
                        .is_some_and(|length| length > 2_000_000)
                    {
                        return Err(IndexIngestError::Csv("download excede 2 MB".into()));
                    }
                    let mut body = Vec::new();
                    while let Some(chunk) = response.chunk().await? {
                        if body.len() + chunk.len() > 2_000_000 {
                            return Err(IndexIngestError::Csv("download excede 2 MB".into()));
                        }
                        body.extend_from_slice(&chunk);
                    }
                    payload = Some(body);
                    break;
                }
                Ok(response)
                    if !response.status().is_server_error()
                        && response.status().as_u16() != 429 =>
                {
                    return Err(IndexIngestError::Csv(format!(
                        "B3 retornou HTTP {}",
                        response.status()
                    )))
                }
                Ok(_) | Err(_) if attempt < 2 => {
                    tokio::time::sleep(Duration::from_secs(1 << attempt)).await
                }
                Ok(response) => {
                    return Err(IndexIngestError::Csv(format!(
                        "B3 retornou HTTP {} após tentativas",
                        response.status()
                    )))
                }
                Err(error) => return Err(IndexIngestError::Http(error)),
            }
        }
        let payload = payload.ok_or_else(|| IndexIngestError::Csv("resposta vazia".into()))?;
        let points = parse_b3_csv(index, year, &payload)?;
        if points.is_empty() {
            return Err(IndexIngestError::Empty {
                index: index.as_str().into(),
                year,
            });
        }
        let rows = points.clone();
        let target_dir = self.data_dir.join("processed/indices");
        let hash = blake3::hash(&payload).to_hex().to_string();
        let active = self
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT path FROM index_partitions WHERE index_code=?1 AND year=?2",
                params![index.as_str(), year],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .map(PathBuf::from);
        let path = target_dir.join(format!(
            "{}-{year}-{hash}.parquet",
            index.as_str().to_lowercase()
        ));
        let writer_hash = hash.clone();
        let old = active.clone();
        let target = path.clone();
        let rows_count = tokio::task::spawn_blocking(move || {
            write_snapshot(&target, index, year, rows, &writer_hash, old.as_deref())
        })
        .await
        .map_err(|e| IndexIngestError::Csv(e.to_string()))??;
        {
            let mut db = self.db.lock().unwrap();
            let tx = db.transaction()?;
            let completed = Utc::now().to_rfc3339();
            tx.execute("INSERT INTO index_partitions(index_code,year,path,source_hash,row_count,completed_at) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(index_code,year) DO UPDATE SET path=excluded.path,source_hash=excluded.source_hash,row_count=excluded.row_count,completed_at=excluded.completed_at",params![index.as_str(),year,path.to_string_lossy(),hash,rows_count as i64,completed])?;
            tx.execute("INSERT INTO index_ingest_years(index_code,year,source_hash,row_count,completed_at) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(index_code,year) DO UPDATE SET source_hash=excluded.source_hash,row_count=excluded.row_count,completed_at=excluded.completed_at", params![index.as_str(), year, hash, rows_count as i64, Utc::now().to_rfc3339()])?;
            tx.commit()?;
        }
        self.loader
            .reload()
            .await
            .map_err(|e| IndexIngestError::Reload(e.to_string()))?;
        Ok(rows_count)
    }

    pub async fn scheduler(self: Arc<Self>) {
        if let Ok(jobs) = self.interrupted_jobs() {
            for (id, request) in jobs {
                let now = Utc::now().with_timezone(&Sao_Paulo).year();
                self.clone()
                    .run_job(
                        id,
                        request.indices.unwrap_or_else(|| {
                            vec![IndexCode::Ibov, IndexCode::Idiv, IndexCode::Ifix]
                        }),
                        request.ano_inicio.unwrap_or(now),
                        request.ano_fim.unwrap_or(now),
                        "resume",
                    )
                    .await;
            }
        }
        let current = Utc::now().with_timezone(&Sao_Paulo).year();
        let init = format!("startup-{current}");
        let _ = self.db.lock().unwrap().execute(
            "INSERT OR IGNORE INTO index_ingest_jobs(id,status,created_at) VALUES(?1,'queued',?2)",
            params![init, Utc::now().to_rfc3339()],
        );
        self.clone()
            .run_job(
                init,
                vec![IndexCode::Ibov, IndexCode::Idiv, IndexCode::Ifix],
                current,
                current,
                "startup",
            )
            .await;
        for index in [IndexCode::Ibov, IndexCode::Idiv, IndexCode::Ifix] {
            for year in index.first_year()..current {
                let done = self
                    .db
                    .lock()
                    .unwrap()
                    .query_row(
                        "SELECT 1 FROM index_ingest_years WHERE index_code=?1 AND year=?2",
                        params![index.as_str(), year],
                        |_| Ok(()),
                    )
                    .is_ok();
                if !done {
                    let id = format!("backfill-{}-{year}", index.as_str().to_lowercase());
                    let _ = self.db.lock().unwrap().execute("INSERT OR IGNORE INTO index_ingest_jobs(id,status,created_at) VALUES(?1,'queued',?2)", params![id,Utc::now().to_rfc3339()]);
                    self.clone()
                        .run_job(id, vec![index], year, year, "backfill")
                        .await;
                }
            }
        }
        loop {
            let now = Utc::now().with_timezone(&Sao_Paulo);
            let today = now.date_naive().to_string();
            let completed = self
                .db
                .lock()
                .unwrap()
                .query_row(
                    "SELECT 1 FROM index_ingest_schedule WHERE local_date=?1",
                    [&today],
                    |_| Ok(()),
                )
                .is_ok();
            if schedule_due(now, completed) {
                for scheduled_year in scheduled_years(now) {
                    let id = format!("scheduled-{scheduled_year}-{today}");
                    let _ = self.db.lock().unwrap().execute("INSERT OR IGNORE INTO index_ingest_jobs(id,status,created_at) VALUES(?1,'queued',?2)", params![id,Utc::now().to_rfc3339()]);
                    self.clone()
                        .run_job(
                            id,
                            vec![IndexCode::Ibov, IndexCode::Idiv, IndexCode::Ifix],
                            scheduled_year,
                            scheduled_year,
                            "scheduled",
                        )
                        .await;
                }
                for index in [IndexCode::Ibov, IndexCode::Idiv, IndexCode::Ifix] {
                    for year in index.first_year()..now.year() {
                        let done = self
                            .db
                            .lock()
                            .unwrap()
                            .query_row(
                                "SELECT 1 FROM index_ingest_years WHERE index_code=?1 AND year=?2",
                                params![index.as_str(), year],
                                |_| Ok(()),
                            )
                            .is_ok();
                        if !done {
                            let id = format!(
                                "scheduled-backfill-{}-{year}-{today}",
                                index.as_str().to_lowercase()
                            );
                            let _=self.db.lock().unwrap().execute("INSERT OR IGNORE INTO index_ingest_jobs(id,status,created_at) VALUES(?1,'queued',?2)",params![id,Utc::now().to_rfc3339()]);
                            self.clone()
                                .run_job(id, vec![index], year, year, "scheduled-backfill")
                                .await;
                        }
                    }
                }
                let _ = self.db.lock().unwrap().execute("INSERT OR IGNORE INTO index_ingest_schedule(local_date,completed_at) VALUES(?1,?2)", params![today,Utc::now().to_rfc3339()]);
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    }
}

fn write_snapshot(
    path: &Path,
    index: IndexCode,
    year: i32,
    mut points: Vec<IndexPoint>,
    hash: &str,
    old_path: Option<&Path>,
) -> Result<usize, IndexIngestError> {
    fs::create_dir_all(path.parent().unwrap())?;
    if points.iter().any(|p| p.date.year() != year) {
        return Err(IndexIngestError::Csv("data fora do ano do arquivo".into()));
    }
    let mut merged: BTreeMap<NaiveDate, i64> = BTreeMap::new();
    if let Some(old_path) = old_path.filter(|p| p.exists()) {
        let reader = ParquetRecordBatchReaderBuilder::try_new(fs::File::open(old_path)?)
            .map_err(|e| IndexIngestError::Csv(e.to_string()))?
            .build()
            .map_err(|e| IndexIngestError::Csv(e.to_string()))?;
        for batch in reader {
            let batch = batch.map_err(|e| IndexIngestError::Csv(e.to_string()))?;
            let dates = batch
                .column(1)
                .as_any()
                .downcast_ref::<Date32Array>()
                .ok_or_else(|| {
                    IndexIngestError::Csv("coluna date inválida na partição existente".into())
                })?;
            let values = batch
                .column(2)
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or_else(|| {
                    IndexIngestError::Csv("coluna close inválida na partição existente".into())
                })?;
            for row in 0..batch.num_rows() {
                let date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
                    + chrono::Duration::days(dates.value(row) as i64);
                merged.insert(date, values.value(row));
            }
        }
    }
    for point in points.drain(..) {
        if let Some(previous) = merged.get(&point.date) {
            if *previous != point.close_cents {
                return Err(IndexIngestError::Csv(format!(
                    "valor conflitante para {}: {} != {} centavos",
                    point.date, previous, point.close_cents
                )));
            }
        }
        merged.insert(point.date, point.close_cents);
    }
    let points: Vec<IndexPoint> = merged
        .into_iter()
        .map(|(date, close_cents)| IndexPoint { date, close_cents })
        .collect();
    let date = Date32Array::from(
        points
            .iter()
            .map(|p| (p.date - NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()).num_days() as i32)
            .collect::<Vec<_>>(),
    );
    let values = Int64Array::from(points.iter().map(|p| p.close_cents).collect::<Vec<_>>());
    let names = StringArray::from(vec![index.as_str(); points.len()]);
    let ingested = TimestampNanosecondArray::from(vec![
        Some(
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );
        points.len()
    ]);
    let source_file =
        StringArray::from(vec![format!("B3:{}:{year}", index.as_str()); points.len()]);
    let hashes = StringArray::from(vec![hash; points.len()]);
    let batch = RecordBatch::try_new(
        index_metrics_schema(),
        vec![
            Arc::new(names),
            Arc::new(date),
            Arc::new(values),
            Arc::new(ingested),
            Arc::new(source_file),
            Arc::new(hashes),
        ],
    )?;
    if path.exists() {
        return Ok(points.len());
    }
    let temp = path.with_extension("parquet.tmp");
    let file = fs::File::create(&temp)?;
    let mut writer = ArrowWriter::try_new(file, index_metrics_schema(), None)?;
    writer.write(&batch)?;
    writer.close()?;
    fs::rename(temp, path)?;
    Ok(points.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn parses_captured_official_files_for_all_indices() {
        for (index, name) in [
            (IndexCode::Ibov, "ibovespa"),
            (IndexCode::Idiv, "idiv"),
            (IndexCode::Ifix, "ifix"),
        ] {
            let bytes =
                fs::read(format!("tests/fixtures/raw/index-metrics/{name}-2025.b64")).unwrap();
            let points = parse_b3_csv(index, 2025, &bytes).unwrap();
            assert!(points.len() > 200);
            assert!(points.iter().all(|point| point.date.year() == 2025));
        }
    }

    #[test]
    fn rejects_bad_dates_and_preserves_exact_cents() {
        let mut csv = String::from(
            "IBOVESPA - 2024\r\nDia;Jan;Fev;Mar;Abr;Mai;Jun;Jul;Ago;Set;Out;Nov;Dez\r\n\r\n",
        );
        for day in 1..=31 {
            csv.push_str(&format!(
                "{day};{}{}\r\n",
                if day == 29 { "123.456,78" } else { "" },
                ";".repeat(11)
            ));
        }
        let encoded = base64::engine::general_purpose::STANDARD.encode(csv.as_bytes());
        let points = parse_b3_csv(IndexCode::Ibov, 2024, encoded.as_bytes()).unwrap();
        assert_eq!(points[0].close_cents, 12_345_678);
        let bad="IBOVESPA - 2023\r\nDia;Jan;Fev;Mar;Abr;Mai;Jun;Jul;Ago;Set;Out;Nov;Dez\r\n\r\n32;1,00;;;;;;;;;;;\r\n";
        let encoded = base64::engine::general_purpose::STANDARD.encode(bad.as_bytes());
        assert!(parse_b3_csv(IndexCode::Ibov, 2023, encoded.as_bytes()).is_err());
    }

    #[test]
    fn snapshots_append_new_dates_and_reject_conflicting_values() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("ibov-2025-a.parquet");
        let newer = directory.path().join("ibov-2025-b.parquet");
        let first = vec![IndexPoint {
            date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            close_cents: 10000,
        }];
        assert_eq!(
            write_snapshot(&original, IndexCode::Ibov, 2025, first.clone(), "a", None).unwrap(),
            1
        );
        let second = vec![IndexPoint {
            date: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
            close_cents: 10100,
        }];
        assert_eq!(
            write_snapshot(&newer, IndexCode::Ibov, 2025, second, "b", Some(&original)).unwrap(),
            2
        );
        let changed = vec![IndexPoint {
            date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            close_cents: 10001,
        }];
        assert!(write_snapshot(
            &directory.path().join("conflict.parquet"),
            IndexCode::Ibov,
            2025,
            changed,
            "c",
            Some(&newer)
        )
        .is_err());
        assert!(original.exists());
        assert!(newer.exists());
        assert!(!directory.path().join("conflict.parquet").exists());
    }

    #[test]
    fn scheduler_uses_sao_paulo_time_and_refreshes_previous_year_on_january_first() {
        use chrono::TimeZone;
        let before = Sao_Paulo
            .with_ymd_and_hms(2026, 6, 1, 1, 59, 0)
            .single()
            .unwrap();
        let due = Sao_Paulo
            .with_ymd_and_hms(2026, 6, 1, 2, 0, 0)
            .single()
            .unwrap();
        assert!(!schedule_due(before, false));
        assert!(schedule_due(due, false));
        assert!(!schedule_due(due, true));
        let jan_first = Sao_Paulo
            .with_ymd_and_hms(2027, 1, 1, 2, 0, 0)
            .single()
            .unwrap();
        assert_eq!(scheduled_years(jan_first), vec![2026, 2027]);
    }

    #[tokio::test]
    async fn ingestor_initializes_ledger_schema_for_a_fresh_data_volume() {
        let directory = tempfile::tempdir().unwrap();
        let config = crate::config::DataConfig {
            dir: directory.path().to_path_buf(),
            reload_interval_seconds: 0,
        };
        let loader = Arc::new(DataLoader::new_standalone(&config).unwrap());

        let _ingestor = IndexIngestor::new(directory.path(), loader.clone()).unwrap();
        loader.reload().await.unwrap();

        assert_eq!(loader.ticker_registry.count(), 0);
    }
}
