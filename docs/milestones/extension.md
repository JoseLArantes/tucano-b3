# Development Plan — Frontend + Consumer Endpoints for `tucano-b3`

## 0. Purpose

This plan extends `tucano-b3` from a read-only market-data API into a **frontend-ready and consumer-ready API** with:

1. Historical ingestion triggering from files already available in `data/raw/`.
2. File discovery by dataset, year, month, and day.
3. Ingestion/parsing job management.
4. Real-time ingestion progress tracking.
5. Frontend-friendly market data endpoints.
6. Consumer-friendly query/export endpoints.
7. Full OpenAPI + Swagger documentation updates.

This plan is written for **LLM-driven development**.

---

## 1. Scope Change

Previous `tucano-b3` assumption:

> `tucano-b3` serves data only. Ingestion is done exclusively by `tucano-b3-cli`.

New requirement:

> `tucano-b3` must expose controlled ingestion endpoints so a frontend can trigger historical ingestion based on files already present in `data/raw/`.

### New rule

`tucano-b3` may trigger ingestion, but it must **not upload files** in this phase.

Files must already exist in:

```txt
data/raw/cotahist/
data/raw/ibov/
```

### Runtime dependency rule

Do **not** shell out to `tucano-b3-cli`.

Required approach:

```txt
tucano-b3-cli parsers / ingestion engine
        ↓ extract into
tucano-core or tucano-ingest
        ↓ used by
tucano-b3 API background worker
```

No external runtime dependency is allowed.

---

## 2. LLM Execution Rules

1. Execute tasks in order.
2. Do not invent endpoints outside this plan.
3. Do not accept arbitrary filesystem paths from API consumers.
4. All ingestion requests must reference `file_id`, not raw paths.
5. All ingestion work runs in background jobs.
6. HTTP requests must not block until ingestion completes.
7. Progress must be observable through persisted job state and SSE.
8. All user-facing OpenAPI descriptions must be in Brazilian Portuguese.
9. Code identifiers must be in English.
10. Add tests for every endpoint, service, and database migration.
11. Maintain 100% coverage for new business logic.
12. Update Swagger/OpenAPI in the same task that adds each endpoint.

---

## 3. New API Surface

### 3.1 File Catalog / Availability

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/catalog/datasets` | List supported datasets |
| GET | `/api/v1/catalog/files` | List raw files available for ingestion |
| GET | `/api/v1/catalog/coverage` | Show available and ingested dates |
| POST | `/api/v1/catalog/scan` | Rescan raw files and update catalog |

### 3.2 Ingestion Jobs

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/ingestion/jobs` | Create ingestion job |
| GET | `/api/v1/ingestion/jobs` | List ingestion jobs |
| GET | `/api/v1/ingestion/jobs/{job_id}` | Get job status |
| POST | `/api/v1/ingestion/jobs/{job_id}/cancel` | Cancel running/queued job |
| GET | `/api/v1/ingestion/jobs/{job_id}/report` | Get quality report |
| GET | `/api/v1/ingestion/jobs/{job_id}/logs` | Get job logs |
| GET | `/api/v1/ingestion/jobs/{job_id}/events` | SSE progress stream |
| GET | `/api/v1/ingestion/events` | SSE stream for all jobs |

### 3.3 Frontend-Friendly Market Data

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/frontend/dashboard` | Home dashboard data |
| GET | `/api/v1/frontend/search` | Search tickers |
| GET | `/api/v1/frontend/market/summary` | Market summary for a date |
| GET | `/api/v1/frontend/market/movers` | Top gainers/losers |
| GET | `/api/v1/frontend/series` | Multi-ticker time series |
| GET | `/api/v1/frontend/compare` | Normalized comparison series |

### 3.4 Consumer Query / Export

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/query/sql` | Safe read-only SQL query |
| GET | `/api/v1/export/cotahist` | Export COTAHIST filtered data |
| GET | `/api/v1/export/ibov` | Export IBOV filtered data |

### 3.5 System / Admin

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/system/status` | Full system status |
| POST | `/api/v1/system/reload` | Reload Parquet files and ticker registry |
| POST | `/api/v1/system/cache/purge` | Purge API cache |

---

## 4. Required New Modules

Create or extend:

```txt
src/
├── api/
│   ├── catalog.rs
│   ├── ingestion.rs
│   ├── frontend.rs
│   ├── query.rs
│   ├── export.rs
│   └── system.rs
├── catalog/
│   ├── mod.rs
│   ├── scanner.rs
│   ├── models.rs
│   └── repository.rs
├── ingestion/
│   ├── mod.rs
│   ├── job.rs
│   ├── manager.rs
│   ├── worker.rs
│   ├── events.rs
│   ├── repository.rs
│   └── error.rs
├── events/
│   ├── mod.rs
│   ├── sse.rs
│   └── stream.rs
├── consumers/
│   ├── mod.rs
│   ├── sql_guard.rs
│   └── export.rs
└── docs/
    ├── mod.rs
    └── examples.rs
```

---

## 5. SQLite Data Model Extensions

Use the existing SQLite ledger:

```txt
data/ledger/tucano.db
```

Add migrations.

### 5.1 `raw_files`

Tracks files discovered in `data/raw/`.

```sql
CREATE TABLE IF NOT EXISTS raw_files (
    id              TEXT PRIMARY KEY,
    dataset         TEXT NOT NULL CHECK (dataset IN ('cotahist', 'ibov')),
    relative_path   TEXT NOT NULL UNIQUE,
    filename        TEXT NOT NULL,
    file_hash       TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    encoding        TEXT NOT NULL DEFAULT 'latin1',

    year            INTEGER,
    month           INTEGER,
    day             INTEGER,

    date_min        TEXT,
    date_max        TEXT,

    scan_status     TEXT NOT NULL CHECK (
        scan_status IN ('discovered', 'scanned', 'invalid', 'missing')
    ),

    rows_estimated  INTEGER,
    error_message   TEXT,

    discovered_at   TEXT NOT NULL,
    scanned_at      TEXT
);
```

### 5.2 `ingestion_jobs`

```sql
CREATE TABLE IF NOT EXISTS ingestion_jobs (
    id                  TEXT PRIMARY KEY,
    dataset             TEXT NOT NULL CHECK (dataset IN ('cotahist', 'ibov', 'all')),
    mode                TEXT NOT NULL CHECK (
        mode IN ('full', 'year', 'month', 'day', 'range')
    ),

    status              TEXT NOT NULL CHECK (
        status IN (
            'queued',
            'running',
            'completed',
            'failed',
            'cancelled'
        )
    ),

    replace_existing    INTEGER NOT NULL DEFAULT 0,
    validate_only       INTEGER NOT NULL DEFAULT 0,

    requested_by        TEXT,
    request_payload     TEXT NOT NULL,

    files_total         INTEGER NOT NULL DEFAULT 0,
    files_done          INTEGER NOT NULL DEFAULT 0,

    rows_total          INTEGER NOT NULL DEFAULT 0,
    rows_valid          INTEGER NOT NULL DEFAULT 0,
    rows_rejected       INTEGER NOT NULL DEFAULT 0,

    progress_pct        REAL NOT NULL DEFAULT 0.0,
    current_file        TEXT,
    current_phase       TEXT,

    error_code          TEXT,
    error_message       TEXT,

    created_at          TEXT NOT NULL,
    started_at          TEXT,
    finished_at         TEXT
);
```

### 5.3 `ingestion_job_files`

```sql
CREATE TABLE IF NOT EXISTS ingestion_job_files (
    job_id          TEXT NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,
    file_id         TEXT NOT NULL REFERENCES raw_files(id),
    status          TEXT NOT NULL CHECK (
        status IN ('queued', 'running', 'completed', 'failed', 'skipped')
    ),

    rows_total      INTEGER NOT NULL DEFAULT 0,
    rows_valid      INTEGER NOT NULL DEFAULT 0,
    rows_rejected   INTEGER NOT NULL DEFAULT 0,

    started_at      TEXT,
    finished_at     TEXT,
    error_message   TEXT,

    PRIMARY KEY (job_id, file_id)
);
```

### 5.4 `ingestion_events`

Persist events so SSE clients can reconnect with `Last-Event-ID`.

```sql
CREATE TABLE IF NOT EXISTS ingestion_events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id          TEXT NOT NULL REFERENCES ingestion_jobs(id) ON DELETE CASCADE,
    event_type      TEXT NOT NULL,
    sequence        INTEGER NOT NULL,
    payload_json    TEXT NOT NULL,
    created_at      TEXT NOT NULL,

    UNIQUE(job_id, sequence)
);
```

### 5.5 Indexes

```sql
CREATE INDEX IF NOT EXISTS idx_raw_files_dataset
ON raw_files(dataset);

CREATE INDEX IF NOT EXISTS idx_raw_files_date
ON raw_files(dataset, year, month, day);

CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_status
ON ingestion_jobs(status);

CREATE INDEX IF NOT EXISTS idx_ingestion_jobs_created
ON ingestion_jobs(created_at);

CREATE INDEX IF NOT EXISTS idx_ingestion_events_job
ON ingestion_events(job_id, id);
```

---

## 6. Task F0 — Extract Ingestion Core

### Goal

Make ingestion callable from the API without depending on the CLI binary.

### Required outcome

Create a reusable crate or module:

```txt
tucano-core
```

or:

```txt
src/ingestion/engine/
```

Preferred long-term structure:

```txt
crates/
├── tucano-core/
│   ├── parsers/
│   ├── writers/
│   ├── ledger/
│   └── quality/
├── tucano-b3-cli/
└── tucano-b3/
```

### Requirements

1. Move reusable COTAHIST parser logic into core.
2. Move reusable IBOV parser logic into core.
3. Move Parquet writer logic into core.
4. Move quality report generation into core.
5. CLI must call core.
6. API worker must call core.
7. No shell execution.
8. No runtime binary dependency.

### Acceptance criteria

- [ ] CLI still ingests files.
- [ ] API can call ingestion engine as Rust function.
- [ ] No `std::process::Command` used for ingestion.
- [ ] Existing CLI tests pass.
- [ ] Existing API tests pass.

---

## 7. Task F1 — Raw File Catalog

### Goal

Allow frontend to discover available historical files by dataset/year/month/day.

### Files

Create:

```txt
src/catalog/models.rs
src/catalog/scanner.rs
src/catalog/repository.rs
src/api/catalog.rs
```

### Models

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct RawFileResponse {
    pub file_id: String,
    pub dataset: String,
    pub filename: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub file_hash: String,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,
    pub date_min: Option<String>,
    pub date_max: Option<String>,
    pub scan_status: String,
}
```

### Scanner rules

For COTAHIST:

1. Scan `data/raw/cotahist/*.TXT`.
2. Detect year from filename if possible.
3. Read header and sample records to detect date range.
4. Do not fully parse the file during catalog scan.
5. Store file metadata in `raw_files`.

For IBOV:

1. Scan `data/raw/ibov/*.csv`.
2. Detect date range by reading first and last valid data rows.
3. Store file metadata in `raw_files`.

### Endpoints

#### `GET /api/v1/catalog/datasets`

Response:

```json
{
  "data": [
    {
      "dataset": "cotahist",
      "nome": "Cotações Históricas B3",
      "formato": "fixed-width TXT",
      "ativo": true
    },
    {
      "dataset": "ibov",
      "nome": "Série Histórica do IBOVESPA",
      "formato": "CSV",
      "ativo": true
    }
  ]
}
```

#### `GET /api/v1/catalog/files`

Query params:

| Param | Type | Required |
|---|---|---|
| `dataset` | `cotahist|ibov` | no |
| `ano` | integer | no |
| `mes` | integer 1–12 | no |
| `dia` | integer 1–31 | no |
| `status` | string | no |

Response:

```json
{
  "data": {
    "files": [
      {
        "file_id": "blake3:abc123",
        "dataset": "cotahist",
        "filename": "COTAHIST_D2026.TXT",
        "relative_path": "raw/cotahist/COTAHIST_D2026.TXT",
        "size_bytes": 123456789,
        "year": 2026,
        "month": null,
        "day": null,
        "date_min": "2026-01-02",
        "date_max": "2026-11-07",
        "scan_status": "scanned"
      }
    ]
  }
}
```

#### `GET /api/v1/catalog/coverage`

Query params:

| Param | Type |
|---|---|
| `dataset` | required |
| `ano` | optional |
| `mes` | optional |

Response:

```json
{
  "data": {
    "dataset": "cotahist",
    "year": 2026,
    "available_days": ["2026-01-02", "2026-01-03"],
    "ingested_days": ["2026-01-02"],
    "missing_days": ["2026-01-03"]
  }
}
```

#### `POST /api/v1/catalog/scan`

Creates catalog rescan job.

Response:

```json
{
  "data": {
    "scan_id": "uuid",
    "status": "queued"
  }
}
```

### Acceptance criteria

- [ ] Frontend can list datasets.
- [ ] Frontend list files by year/month/day.
- [ ] API never accepts arbitrary paths.
- [ ] File IDs are stable.
- [ ] Catalog scan does not fully ingest files.
- [ ] OpenAPI includes all catalog schemas.

---

## 8. Task F2 — Ingestion Job Manager

### Goal

Create background ingestion jobs.

### Files

Create:

```txt
src/ingestion/job.rs
src/ingestion/manager.rs
src/ingestion/worker.rs
src/ingestion/events.rs
src/ingestion/repository.rs
```

### Job request model

```rust
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateIngestionJobRequest {
    pub dataset: Dataset,
    pub mode: IngestionMode,

    pub file_ids: Vec<String>,

    pub year: Option<i32>,
    pub month: Option<u32>,
    pub day: Option<u32>,

    pub desde: Option<String>,
    pub ate: Option<String>,

    #[serde(default)]
    pub replace_existing: bool,

    #[serde(default)]
    pub validate_only: bool,
}
```

### Enums

```rust
#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Dataset {
    Cotahist,
    Ibov,
    All,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum IngestionMode {
    Full,
    Year,
    Month,
    Day,
    Range,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}
```

### Manager requirements

1. Validate request.
2. Resolve `file_ids` to catalog files.
3. Reject unknown file IDs.
4. Reject files outside `data/raw/`.
5. Create job in SQLite.
6. Queue job in memory.
7. Return immediately with `202 Accepted`.

### Worker requirements

1. One job runs at a time by default.
2. Worker updates job state in SQLite.
3. Worker emits progress events.
4. Worker calls ingestion core.
5. Worker writes Parquet outputs.
6. Worker updates ledger.
7. Worker triggers data reload.
8. Worker purges cache after successful ingestion.
9. Worker supports cancellation.

### Acceptance criteria

- [ ] `POST /ingestion/jobs` returns immediately.
- [ ] Job persists in SQLite.
- [ ] Worker processes queued jobs.
- [ ] Job status transitions correctly.
- [ ] Failed jobs store error details.
- [ ] Cancelled jobs stop safely.
- [ ] Successful jobs reload API data.

---

## 9. Task F3 — Ingestion API Endpoints

### Files

Create:

```txt
src/api/ingestion.rs
```

Modify:

```txt
src/api/mod.rs
src/api/openapi.rs
```

### Endpoints

#### `POST /api/v1/ingestion/jobs`

Request examples:

Full file ingestion:

```json
{
  "dataset": "cotahist",
  "mode": "full",
  "file_ids": ["blake3:abc123"],
  "replace_existing": false,
  "validate_only": false
}
```

Year ingestion:

```json
{
  "dataset": "cotahist",
  "mode": "year",
  "year": 2026,
  "file_ids": ["blake3:abc123"]
}
```

Month ingestion:

```json
{
  "dataset": "cotahist",
  "mode": "month",
  "year": 2026,
  "month": 11,
  "file_ids": ["blake3:abc123"]
}
```

Day ingestion:

```json
{
  "dataset": "cotahist",
  "mode": "day",
  "year": 2026,
  "month": 11,
  "day": 7,
  "file_ids": ["blake3:abc123"]
}
```

Response:

```json
{
  "data": {
    "job_id": "uuid",
    "status": "queued",
    "dataset": "cotahist",
    "mode": "month",
    "created_at": "2026-11-11T12:00:00Z"
  }
}
```

HTTP status:

```txt
202 Accepted
```

#### `GET /api/v1/ingestion/jobs`

Query params:

| Param | Type |
|---|---|
| `status` | optional |
| `dataset` | optional |
| `pagina` | optional |
| `por_pagina` | optional |

#### `GET /api/v1/ingestion/jobs/{job_id}`

Response:

```json
{
  "data": {
    "job_id": "uuid",
    "dataset": "cotahist",
    "mode": "month",
    "status": "running",
    "progress_pct": 42.5,
    "current_file": "COTAHIST_D2026.TXT",
    "current_phase": "parsing",
    "files_total": 1,
    "files_done": 0,
    "rows_total": 5000000,
    "rows_valid": 4999980,
    "rows_rejected": 20
  }
}
```

#### `POST /api/v1/ingestion/jobs/{job_id}/cancel`

Response:

```json
{
  "data": {
    "job_id": "uuid",
    "status": "cancelled"
  }
}
```

#### `GET /api/v1/ingestion/jobs/{job_id}/report`

Returns quality report JSON.

#### `GET /api/v1/ingestion/jobs/{job_id}/logs`

Returns paginated logs.

### Acceptance criteria

- [ ] Create job endpoint works.
- [ ] List jobs endpoint works.
- [ ] Get job endpoint works.
- [ ] Cancel job endpoint works.
- [ ] Report endpoint works.
- [ ] Logs endpoint works.
- [ ] Unknown job returns 404.
- [ ] Invalid mode/date combination returns 400.
- [ ] All endpoints documented in OpenAPI.

---

## 10. Task F4 — Real-Time Progress via SSE

### Goal

Allow frontend and consumers to follow ingestion/parsing progress.

### Use SSE, not WebSocket

Reason:

1. Simpler HTTP semantics.
2. Works with proxies.
3. Native browser `EventSource`.
4. Reconnect support via `Last-Event-ID`.
5. No bidirectional protocol needed.

### Files

Create:

```txt
src/events/sse.rs
src/events/stream.rs
```

Modify:

```txt
src/api/ingestion.rs
src/ingestion/events.rs
```

### Endpoints

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/ingestion/jobs/{job_id}/events` | Events for one job |
| GET | `/api/v1/ingestion/events` | Events for all jobs |

### Event model

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct IngestionEvent {
    pub event_id: i64,
    pub job_id: String,
    pub event_type: String,
    pub sequence: i64,
    pub timestamp: String,
    pub payload: serde_json::Value,
}
```

### Event types

```txt
job.created
job.queued
job.started
file.started
file.progress
file.completed
quality.issue
job.progress
job.completed
job.failed
job.cancelled
heartbeat
```

### Example SSE stream

```txt
id: 101
event: job.started
data: {"job_id":"uuid","status":"running"}

id: 102
event: file.started
data: {"job_id":"uuid","filename":"COTAHIST_D2026.TXT"}

id: 103
event: job.progress
data: {"job_id":"uuid","progress_pct":42.5,"rows_valid":5000000}

id: 104
event: job.completed
data: {"job_id":"uuid","status":"completed"}
```

### Reconnect behavior

1. Client sends `Last-Event-ID`.
2. Server replays persisted events after that ID from SQLite.
3. Server then subscribes to live broadcast.
4. Heartbeat every 15 seconds.

### Acceptance criteria

- [ ] Frontend can connect with `EventSource`.
- [ ] Events are persisted.
- [ ] Reconnect replays missing events.
- [ ] Heartbeat keeps connection alive.
- [ ] Completed job stream closes after final event.
- [ ] Global stream works.
- [ ] OpenAPI documents `text/event-stream`.

---

## 11. Task F5 — Frontend-Friendly Read Endpoints

### Goal

Provide optimized endpoints for common frontend screens.

### Files

Create:

```txt
src/api/frontend.rs
src/services/frontend.rs
```

### Endpoints

#### `GET /api/v1/frontend/dashboard`

Returns:

```json
{
  "data": {
    "ibov_latest": {},
    "market_summary": {
      "date": "2026-11-07",
      "tickers_count": 487,
      "gainers": 241,
      "losers": 210,
      "unchanged": 36,
      "total_volume": 12345678900
    },
    "top_gainers": [],
    "top_losers": [],
    "most_traded": [],
    "data_coverage": {
      "cotahist_min": "2000-01-03",
      "cotahist_max": "2026-11-07",
      "ibov_min": "1994-01-03",
      "ibov_max": "2026-11-07"
    }
  }
}
```

#### `GET /api/v1/frontend/search?q=VAL`

Returns ticker search results.

#### `GET /api/v1/frontend/market/summary?date=2026-11-07`

Returns gainers/losers/unchanged counts and volume.

#### `GET /api/v1/frontend/market/movers`

Query params:

| Param | Values |
|---|---|
| `date` | YYYY-MM-DD |
| `direction` | `gainers`, `losers`, `volume` |
| `limit` | 1–100 |

#### `GET /api/v1/frontend/series`

Query params:

| Param | Example |
|---|---|
| `tickers` | `VALE3,PETR4,ITUB4` |
| `desde` | `2026-01-01` |
| `ate` | `2026-11-07` |
| `campos` | `data,pre_ult,vol_tot` |

Returns chart-ready series:

```json
{
  "data": {
    "series": [
      {
        "ticker": "VALE3",
        "points": [
          {"data": "2026-01-02", "pre_ult": 62.17}
        ]
      }
    ]
  }
}
```

#### `GET /api/v1/frontend/compare`

Same as series, but normalized to 100 at first date.

```json
{
  "data": {
    "base_date": "2026-01-02",
    "series": [
      {
        "ticker": "VALE3",
        "points": [
          {"data": "2026-01-02", "value": 100.0},
          {"data": "2026-01-03", "value": 101.2}
        ]
      }
    ]
  }
}
```

### Acceptance criteria

- [ ] Dashboard endpoint powers a homepage.
- [ ] Search endpoint powers autocomplete.
- [ ] Movers endpoint powers ranking tables.
- [ ] Series endpoint powers charts.
- [ ] Compare endpoint powers normalized performance charts.
- [ ] All endpoints cached.
- [ ] All endpoints documented.

---

## 12. Task F6 — Consumer SQL and Export Endpoints

### Goal

Allow advanced consumers to query/export data safely.

### Files

Create:

```txt
src/api/query.rs
src/api/export.rs
src/consumers/sql_guard.rs
src/consumers/export.rs
```

### SQL endpoint

#### `POST /api/v1/query/sql`

Request:

```json
{
  "sql": "SELECT ticker, AVG(pre_ult) FROM cotahist GROUP BY ticker",
  "limit": 1000,
  "timeout_ms": 5000
}
```

Rules:

1. Only `SELECT` allowed.
2. Reject `INSERT`, `UPDATE`, `DELETE`, `DROP`, `CREATE`, `ALTER`, `COPY`.
3. Reject multiple statements.
4. Max rows: 10,000.
5. Max timeout: 10,000 ms.
6. Only registered tables: `cotahist`, `ibov`.
7. No filesystem access.
8. No external table functions.

Response:

```json
{
  "data": {
    "columns": ["ticker", "avg_pre_ult"],
    "rows": [
      ["VALE3", 62.17]
    ],
    "row_count": 1
  }
}
```

### Export endpoints

#### `GET /api/v1/export/cotahist`

Query params:

| Param | Type |
|---|---|
| `ticker` | optional |
| `tickers` | comma-separated |
| `desde` | optional |
| `ate` | optional |
| `formato` | `csv|json|ndjson|parquet` |

#### `GET /api/v1/export/ibov`

Same date filters and formats.

### Acceptance criteria

- [ ] SQL endpoint executes safe SELECT queries.
- [ ] SQL injection cannot mutate data.
- [ ] Query timeout enforced.
- [ ] Row limit enforced.
- [ ] CSV export works.
- [ ] JSON export works.
- [ ] NDJSON export works.
- [ ] Parquet export works if feasible.
- [ ] OpenAPI documents all formats.

---

## 13. Task F7 — System/Admin Endpoints

### Goal

Allow frontend admin screen to reload data and inspect status.

### Files

Create:

```txt
src/api/system.rs
```

### Endpoints

#### `GET /api/v1/system/status`

Returns:

```json
{
  "data": {
    "version": "0.1.0",
    "uptime_seconds": 3600,
    "data_files_loaded": 3,
    "tickers_count": 1247,
    "cache": {
      "enabled": true,
      "entries": 100,
      "hit_rate_pct": 82.5
    },
    "ingestion": {
      "queued_jobs": 1,
      "running_jobs": 1,
      "failed_jobs": 0
    }
  }
}
```

#### `POST /api/v1/system/reload`

Triggers data reload.

Response:

```json
{
  "data": {
    "status": "completed",
    "files_loaded": 3
  }
}
```

#### `POST /api/v1/system/cache/purge`

Response:

```json
{
  "data": {
    "status": "completed",
    "entries_removed": 100
  }
}
```

### Security

These endpoints require admin API key scope.

---

## 14. Task F8 — API Key Scopes

### Goal

Protect ingestion/admin endpoints.

### Modify API key file

```toml
[[keys]]
key = "tucano-admin-key"
name = "Frontend Admin"
scopes = ["read", "ingest", "admin"]
rate_limit = 1000

[[keys]]
key = "tucano-reader-key"
name = "Read Only Consumer"
scopes = ["read"]
rate_limit = 500
```

### Scope rules

| Endpoint type | Required scope |
|---|---|
| Market data read | `read` |
| SQL query | `read` |
| Export | `read` |
| Catalog read | `read` |
| Catalog scan | `ingest` |
| Create ingestion job | `ingest` |
| Cancel ingestion job | `ingest` |
| System reload | `admin` |
| Cache purge | `admin` |

### Acceptance criteria

- [ ] Read-only key cannot trigger ingestion.
- [ ] Ingest key can trigger ingestion.
- [ ] Admin key can reload/purge.
- [ ] Missing key returns 401 for protected endpoints.
- [ ] Insufficient scope returns 403.
- [ ] OpenAPI documents scopes.

---

## 15. Task F9 — OpenAPI + Swagger Updates

### Goal

Every new endpoint must be fully documented.

### Files

Modify:

```txt
src/api/openapi.rs
src/api/catalog.rs
src/api/ingestion.rs
src/api/frontend.rs
src/api/query.rs
src/api/export.rs
src/api/system.rs
```

Create:

```txt
src/docs/examples.rs
```

### New OpenAPI tags

```rust
tags(
    (name = "catalogo", description = "Descoberta de arquivos e cobertura histórica"),
    (name = "ingestao", description = "Gerenciamento de jobs de ingestão e parsing"),
    (name = "eventos", description = "Acompanhamento em tempo real via SSE"),
    (name = "frontend", description = "Endpoints otimizados para telas de frontend"),
    (name = "consumidores", description = "Consultas SQL e exportação de dados"),
    (name = "sistema", description = "Administração e status do serviço"),
)
```

### Required schemas

Add to OpenAPI components:

```txt
RawFileResponse
DatasetResponse
CoverageResponse
CreateIngestionJobRequest
IngestionJobResponse
IngestionJobListResponse
IngestionEvent
IngestionReportResponse
IngestionLogResponse
DashboardResponse
SearchResponse
MarketSummaryResponse
MarketMoversResponse
SeriesResponse
CompareResponse
SqlQueryRequest
SqlQueryResponse
SystemStatusResponse
ReloadResponse
CachePurgeResponse
```

### SSE documentation

Document:

```txt
Content-Type: text/event-stream
```

Event types:

```txt
job.created
job.started
job.progress
file.started
file.completed
job.completed
job.failed
job.cancelled
heartbeat
```

Swagger may not render SSE interactively, but the spec must describe it.

### OpenAPI example for job creation

```rust
#[utoipa::path(
    post,
    path = "/ingestion/jobs",
    tag = "ingestao",
    summary = "Criar job de ingestão",
    description = "Cria um job assíncrono para ingestão de arquivos históricos já disponíveis em data/raw/. A requisição retorna imediatamente com job_id.",
    request_body = CreateIngestionJobRequest,
    responses(
        (status = 202, description = "Job criado e enfileirado", body = ApiResponse<IngestionJobResponse>),
        (status = 400, description = "Requisição inválida", body = ErrorResponse),
        (status = 401, description = "API key ausente", body = ErrorResponse),
        (status = 403, description = "Permissão insuficiente", body = ErrorResponse),
        (status = 429, description = "Limite de requisições excedido", body = ErrorResponse),
    ),
    security(("ApiKeyAuth" = []))
)]
```

### Acceptance criteria

- [ ] All new endpoints appear in Swagger.
- [ ] All request bodies documented.
- [ ] All response schemas documented.
- [ ] SSE documented.
- [ ] API key scopes documented.
- [ ] Portuguese descriptions used.
- [ ] Examples included for job creation and progress events.

---

## 16. Task F10 — Frontend Contract Documentation

### Goal

Create documentation specifically for frontend developers.

### Files

Create:

```txt
FRONTEND_API.md
```

Language:

```txt
Brazilian Portuguese
```

### Sections

1. Authentication.
2. How to list available files.
3. How to trigger ingestion.
4. How to follow progress with SSE.
5. How to handle job failure.
6. How to refresh market data after ingestion.
7. Dashboard endpoint usage.
8. Chart endpoint usage.
9. Error codes.
10. TypeScript types.

### Include TypeScript types

```ts
export type Dataset = "cotahist" | "ibov" | "all";

export type IngestionMode = "full" | "year" | "month" | "day" | "range";

export type JobStatus =
  | "queued"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface IngestionJob {
  job_id: string;
  dataset: Dataset;
  mode: IngestionMode;
  status: JobStatus;
  progress_pct: number;
  current_file?: string;
  current_phase?: string;
}
```

### Include EventSource example

```ts
const events = new EventSource(
  `/api/v1/ingestion/jobs/${jobId}/events`
);

events.addEventListener("job.progress", (event) => {
  const data = JSON.parse(event.data);
  setProgress(data.progress_pct);
});

events.addEventListener("job.completed", () => {
  events.close();
  refreshDashboard();
});

events.addEventListener("job.failed", (event) => {
  const data = JSON.parse(event.data);
  showError(data.error_message);
  events.close();
});
```

### Acceptance criteria

- [ ] Frontend developer can implement ingestion screen from this doc.
- [ ] Frontend developer can implement progress bar from this doc.
- [ ] Frontend developer can implement dashboard from this doc.
- [ ] Examples are copy-paste usable.

---

## 17. Task F11 — Consumer Documentation

### Goal

Create documentation for API consumers.

### Files

Create:

```txt
CONSUMERS_API.md
```

Language:

```txt
Brazilian Portuguese
```

### Sections

1. API keys.
2. Rate limits.
3. Market data endpoints.
4. SQL endpoint.
5. Export endpoints.
6. Pagination.
7. Error handling.
8. Caching behavior.
9. Python examples.
10. curl examples.

### Python example

```python
import requests

API = "http://localhost:8080/api/v1"
HEADERS = {"X-API-Key": "tucano-reader-key"}

response = requests.get(
    f"{API}/cotahist/tickers/VALE3/quotes",
    params={"dias": 30},
    headers=HEADERS,
)

response.raise_for_status()
data = response.json()["data"]
```

### SQL example

```python
payload = {
    "sql": "SELECT ticker, AVG(pre_ult) FROM cotahist GROUP BY ticker",
    "limit": 1000,
    "timeout_ms": 5000,
}

response = requests.post(
    f"{API}/query/sql",
    json=payload,
    headers=HEADERS,
)
```

### Acceptance criteria

- [ ] Consumer can authenticate.
- [ ] Consumer can fetch quotes.
- [ ] Consumer can run SQL.
- [ ] Consumer can export CSV.
- [ ] Consumer can handle 429.

---

## 18. Task F12 — Testing Plan

### Unit tests

Required for:

```txt
catalog scanner
catalog repository
job manager
job worker
job repository
SSE replay
SQL guard
export service
scope validation
```

### Integration tests

Create:

```txt
tests/integration/catalog.rs
tests/integration/ingestion_jobs.rs
tests/integration/ingestion_sse.rs
tests/integration/frontend_endpoints.rs
tests/integration/query_export.rs
tests/integration/system_admin.rs
tests/integration/openapi_frontend.rs
```

### Required scenarios

1. List empty catalog.
2. Scan catalog with COTAHIST file.
3. Scan catalog with IBOV file.
4. Create ingestion job.
5. Reject unknown file ID.
6. Reject path traversal.
7. Follow job via SSE.
8. Reconnect SSE with `Last-Event-ID`.
9. Cancel queued job.
10. Cancel running job.
11. Failed job emits failure event.
12. Successful job reloads data.
13. Frontend dashboard returns valid response.
14. SQL endpoint rejects non-SELECT.
15. Export CSV works.
16. Admin endpoint rejects read-only key.
17. OpenAPI contains all new endpoints.

### Acceptance criteria

- [ ] All integration tests pass.
- [ ] SSE tests verify event order.
- [ ] Job state transitions tested.
- [ ] Security tests verify scopes.
- [ ] 100% coverage on new business logic.

---

## 19. Task F13 — Final Verification

### Commands

```bash
cargo test --all-features --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo tarpaulin --out Html
```

### Manual frontend flow

```bash
# 1. Scan files
curl -X POST http://localhost:8080/api/v1/catalog/scan \
  -H "X-API-Key: tucano-admin-key"

# 2. List files
curl http://localhost:8080/api/v1/catalog/files?dataset=cotahist \
  -H "X-API-Key: tucano-admin-key"

# 3. Create ingestion job
curl -X POST http://localhost:8080/api/v1/ingestion/jobs \
  -H "Content-Type: application/json" \
  -H "X-API-Key: tucano-admin-key" \
  -d '{
    "dataset": "cotahist",
    "mode": "month",
    "year": 2026,
    "month": 11,
    "file_ids": ["blake3:abc123"],
    "replace_existing": false,
    "validate_only": false
  }'

# 4. Follow progress
curl -N http://localhost:8080/api/v1/ingestion/jobs/{job_id}/events \
  -H "X-API-Key: tucano-admin-key"

# 5. Check job status
curl http://localhost:8080/api/v1/ingestion/jobs/{job_id} \
  -H "X-API-Key: tucano-admin-key"

# 6. Fetch dashboard
curl http://localhost:8080/api/v1/frontend/dashboard \
  -H "X-API-Key: tucano-reader-key"

# 7. Open Swagger
open http://localhost:8080/api/docs/
```

### Final checklist

| # | Criterion | Status |
|---|---|---|
| 1 | Frontend can list available files | ☐ |
| 2 | Frontend can filter files by year/month/day | ☐ |
| 3 | Frontend can trigger ingestion | ☐ |
| 4 | Ingestion runs asynchronously | ☐ |
| 5 | Frontend can follow progress via SSE | ☐ |
| 6 | SSE reconnect works | ☐ |
| 7 | Job can be cancelled | ☐ |
| 8 | Failed jobs expose useful errors | ☐ |
| 9 | Successful ingestion reloads API data | ☐ |
| 10 | Cache purged after ingestion | ☐ |
| 11 | Dashboard endpoint works | ☐ |
| 12 | Search endpoint works | ☐ |
| 13 | Chart series endpoint works | ☐ |
| 14 | SQL endpoint is read-only and safe | ☐ |
| 15 | Export endpoints work | ☐ |
| 16 | Admin endpoints require admin scope | ☐ |
| 17 | OpenAPI includes all new endpoints | ☐ |
| 18 | Swagger UI updated | ☐ |
| 19 | FRONTEND_API.md complete | ☐ |
| 20 | CONSUMERS_API.md complete | ☐ |
| 21 | All tests pass | ☐ |
| 22 | 100% coverage on new logic | ☐ |

---

## 20. Exit Gate

This feature set is complete when:

> A frontend user can open an ingestion screen, see available COTAHIST and IBOV files grouped by year/month/day, trigger ingestion for a selected period, watch parsing progress in real time, see completion or failure, and immediately query refreshed market data. API consumers can safely query and export data, follow ingestion jobs through SSE, and understand all behavior through Swagger and Portuguese documentation.

When this is true, the frontend/consumer layer is ready.