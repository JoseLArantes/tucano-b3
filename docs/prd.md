# Product Requirements Document: `tucano-b3`

## Document Metadata

| Field | Value |
|---|---|
| **Product** | `tucano-b3` |
| **Codename** | Tucano API |
| **Type** | Spinoff of `tucano-b3-cli` |
| **Version** | 1.0.0 |
| **Status** | Draft |
| **Date** | 2026-11-10 |
| **Author** | Principal Design Architect |
| **Document language** | English (for LLM consumption) |
| **Project language** | Brazilian Portuguese (user-facing: API docs, error messages, README) |
| **License** | MIT / Apache-2.0 dual |

---

## 1. Executive Summary

`tucano-b3` is a **high-performance REST API service** written in Rust that serves Brazilian stock market data from B3 (Brasil, Bolsa, Balcão). It exposes two datasets via HTTP:

1. **COTAHIST** — daily historical quotes for all B3-listed instruments.
2. **Séries Históricas do IBOVESPA** — the IBOV index historical series.

The service reads from Parquet files produced by `tucano-b3-cli` (the ingestion tool), caches hot queries in memory, and serves responses with sub-millisecond latency for cached data and sub-100ms for cold queries. It is designed for **high throughput** (10,000+ requests/second on commodity hardware) and **low latency** (p99 < 50ms).

This is a **spinoff** of `tucano-b3-cli`. The CLI handles data ingestion and local querying. `tucano-b3` handles networked, multi-client, production-grade data serving.

---

## 2. Relationship to `tucano-b3-cli`

```
┌─────────────────────────────────────────────────────────────────────┐
│                     tucano-b3-cli (ingestion)                       │
│                                                                     │
│  data/raw/ ──► parsers ──► Parquet + SQLite ledger                  │
│                                                                     │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               │  data/processed/*.parquet
                               │  data/ledger/tucano.db
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     tucano-b3 (REST API)                            │
│                                                                     │
│  Parquet files ──► DataFusion ──► cache ──► HTTP responses          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Key relationship rules:**

| Aspect | Rule |
|---|---|
| Data format | `tucano-b3` reads the exact Parquet schema produced by `tucano-b3-cli` |
| Data location | Configurable `data/` directory (same structure as CLI) |
| Ingestion | `tucano-b3` does NOT ingest data. It only serves. Use `tucano-b3-cli` for ingestion. |
| Schema coupling | If `tucano-b3-cli` changes the Parquet schema, `tucano-b3` must be updated |
| Shared code | Core parsing/encoding/decimal logic can be extracted into a shared crate `tucano-core` |
| Versioning | `tucano-b3` version tracks `tucano-b3-cli` major version |

---

## 3. Vision

> A production-grade, horizontally-scalable REST API that serves B3 market data with sub-millisecond latency for cached queries, sub-100ms for cold queries, and 10,000+ requests/second throughput — all in a single Rust binary with zero runtime dependencies beyond the data files.

---

## 4. Scope

### 4.1 In Scope

| Feature | Description |
|---|---|
| COTAHIST REST endpoints | Ticker lookup, quote history, latest quotes, date-range queries |
| IBOV REST endpoints | Historical series, latest data point, date-range queries |
| In-memory caching | Hot query cache with TTL and LRU eviction |
| Rate limiting | Per-API-key and per-IP rate limiting |
| OpenAPI documentation | Auto-generated Swagger/OpenAPI 3.0 spec |
| Health check | `/health` endpoint for load balancers |
| Prometheus metrics | `/metrics` endpoint for monitoring |
| Structured logging | JSON logs via `tracing` |
| Graceful shutdown | Handle SIGTERM/SIGINT cleanly |
| Docker deployment | Single-container deployment |
| API versioning | `/api/v1/` prefix for all endpoints |

### 4.2 Out of Scope (Deferred to v2+)

- WebSocket streaming
- GraphQL API
- User authentication (OAuth2, JWT)
- Multi-tenancy
- Write endpoints (data ingestion via API)
- Admin panel / dashboard
- Other B3 formats (futures, options, corporate actions)
- Internationalization beyond Portuguese error messages
- Distributed caching (Redis) — v1 uses in-memory only

---

## 5. Goals & Non-Goals

### 5.1 Goals

| ID | Goal |
|---|---|
| G1 | Serve COTAHIST and IBOV data via REST API with sub-100ms p99 latency. |
| G2 | Handle 10,000+ requests/second on a 4-core machine with in-memory cache. |
| G3 | Zero runtime dependencies. Single static binary + Parquet files. |
| G4 | In-memory cache with configurable TTL and max size. |
| G5 | Rate limiting per API key and per IP. |
| G6 | OpenAPI 3.0 spec auto-generated from code. |
| G7 | Prometheus metrics endpoint for production monitoring. |
| G8 | Structured JSON logging for production observability. |
| G9 | Graceful shutdown on SIGTERM/SIGINT. |
| G10 | Docker deployment with health check. |
| G11 | All user-facing API error messages in Brazilian Portuguese. |
| G12 | 100% unit test coverage on business logic. |
| G13 | Reuse Parquet schema from `tucano-b3-cli` without modification. |

### 5.2 Non-Goals

| ID | Non-Goal |
|---|---|
| NG1 | No data ingestion. Use `tucano-b3-cli` for that. |
| NG2 | No database server. Data comes from Parquet files. |
| NG3 | No GUI / dashboard. |
| NG4 | No paid-data support. |
| NG5 | No WebSocket in v1. |
| NG6 | No GraphQL in v1. |
| NG7 | No user authentication in v1 (API keys only for rate limiting). |
| NG8 | No distributed caching in v1. |

---

## 6. Architecture

### 6.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         tucano-b3 (REST API)                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  HTTP Layer (axum)                                                  │
│  ─────────────────────────────────────────────────────────────────  │
│  Router                                                             │
│   ├─ /api/v1/cotahist/*                                             │
│   ├─ /api/v1/ibov/*                                                 │
│   ├─ /api/v1/health                                                 │
│   ├─ /api/v1/metrics                                                │
│   └─ /api/v1/info                                                   │
│  ─────────────────────────────────────────────────────────────────  │
│  Middleware Stack (tower)                                           │
│   ├─ Rate limiter (per API key / per IP)                            │
│   ├─ Request ID generator                                           │
│   ├─ CORS handler                                                   │
│   ├─ Tracing / logging                                              │
│   └─ Compression (zstd/gzip)                                        │
│  ─────────────────────────────────────────────────────────────────  │
│  Service Layer                                                      │
│   ├─ CotahistService    (ticker lookup, quote queries)              │
│   ├─ IbovService        (historical series queries)                 │
│   └─ HealthService      (health check, info)                        │
│  ─────────────────────────────────────────────────────────────────  │
│  Data Layer                                                         │
│   ├─ ParquetReader      (DataFusion, streaming)                     │
│   ├─ QueryCache         (DashMap, TTL + LRU)                        │
│   └─ SchemaRegistry     (Parquet schema validation)                 │
│  ─────────────────────────────────────────────────────────────────  │
│  Infrastructure                                                     │
│   ├─ Config             (TOML config file)                          │
│   ├─ Metrics            (Prometheus exporter)                       │
│   ├─ Logging            (tracing, JSON format)                      │
│   └─ Shutdown           (graceful SIGTERM/SIGINT)                   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
         │
         │ reads
         ▼
┌─────────────────────────────────────────────────────────────────────┐
│  data/processed/                                                    │
│  ├── cotahist/                                                      │
│  │   ├── cotahist-2024.parquet                                     │
│  │   ├── cotahist-2025.parquet                                     │
│  │   └── cotahist-2026.parquet                                     │
│  ├── ibov/                                                          │
│  │   └── ibov-historical.parquet                                   │
│  └── ledger/                                                        │
│      └── tucano.db                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

### 6.2 Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| **Web framework** | `axum` 0.7+ | Tower-based, async, modern, excellent middleware ecosystem |
| **Async runtime** | `tokio` 1.x | Industry standard, multi-threaded |
| **Serialization** | `serde` + `serde_json` | Fast, ergonomic, ecosystem standard |
| **DataFusion** | `datafusion` 42+ | Embedded SQL engine for Parquet queries |
| **Arrow** | `arrow` 53+ | Columnar data model |
| **Parquet** | `parquet` 53+ | File format reader |
| **Cache** | `dashmap` 6+ | Concurrent in-memory cache |
| **Rate limiting** | `tower` + custom | Token bucket per API key |
| **OpenAPI** | `utoipa` 4+ | Auto-generated from code annotations |
| **Metrics** | `prometheus` 0.13+ | Prometheus exposition format |
| **Logging** | `tracing` + `tracing-subscriber` | Structured JSON logs |
| **Config** | `config` crate + TOML | Layered configuration |
| **HTTP client** (tests) | `reqwest` | For integration tests |
| **Compression** | `tower-http` | zstd/gzip response compression |

### 6.3 Crate Dependencies

```toml
[dependencies]
# Web framework
axum = { version = "0.7", features = ["macros"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["compression-full", "cors", "trace", "request-id"] }
hyper = { version = "1.0", features = ["full"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Data layer
datafusion = "42"
arrow = "53"
parquet = "53"
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }

# Cache
dashmap = "6"
lru = "0.12"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# OpenAPI
utoipa = { version = "4", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "7", features = ["axum"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
prometheus = "0.13"

# Config
config = "0.14"
toml = "0.8"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.10"
uuid = { version = "1", features = ["v4"] }
thiserror = "1"
anyhow = "1"

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
tempfile = "3"
assert_cmd = "2"
predicates = "3"
criterion = "0.5"
tokio-test = "0.4"
```

---

## 7. API Design

### 7.1 Base URL

```
http://{host}:{port}/api/v1/
```

Default: `http://localhost:8080/api/v1/`

### 7.2 Common Response Envelope

All API responses use a consistent envelope:

**Success:**
```json
{
  "data": { ... },
  "meta": {
    "request_id": "uuid-v4",
    "timestamp": "2026-11-10T14:32:01-03:00",
    "cached": true,
    "query_time_ms": 0.42
  }
}
```

**Error:**
```json
{
  "error": {
    "code": "TICKER_NOT_FOUND",
    "message": "Ticker não encontrado: XYZXYZ",
    "details": "Verifique se o ticker existe nos dados ingeridos.",
    "request_id": "uuid-v4"
  }
}
```

**Error codes:**

| Code | HTTP Status | Portuguese Message |
|---|---|---|
| `TICKER_NOT_FOUND` | 404 | Ticker não encontrado: {ticker} |
| `INVALID_DATE` | 400 | Formato de data inválido: {value} |
| `DATE_RANGE_INVALID` | 400 | Intervalo de datas inválido |
| `NO_DATA_IN_RANGE` | 404 | Nenhum dado encontrado no período |
| `RATE_LIMITED` | 429 | Limite de requisições excedido |
| `INTERNAL_ERROR` | 500 | Erro interno do servidor |
| `SERVICE_UNAVAILABLE` | 503 | Serviço indisponível |

### 7.3 COTAHIST Endpoints

#### `GET /api/v1/cotahist/tickers`

List all available tickers.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `pagina` | u32 | 1 | Page number |
| `por_pagina` | u32 | 50 | Items per page (max 200) |
| `busca` | string | — | Filter by ticker prefix |
| `ativo` | bool | true | Only active tickers |

**Response:**
```json
{
  "data": {
    "tickers": [
      {
        "ticker": "VALE3",
        "isin": "BRVALEACNPA3",
        "company_name": "VALE S.A.",
        "first_seen": "2000-01-03",
        "last_seen": "2026-11-07",
        "is_active": true
      }
    ],
    "total": 1247,
    "pagina": 1,
    "por_pagina": 50
  }
}
```

#### `GET /api/v1/cotahist/tickers/{ticker}`

Get metadata for a single ticker.

**Response:**
```json
{
  "data": {
    "ticker": "VALE3",
    "isin": "BRVALEACNPA3",
    "company_name": "VALE S.A.",
    "first_seen": "2000-01-03",
    "last_seen": "2026-11-07",
    "is_active": true,
    "total_trading_days": 6842,
    "latest_price": 62.17,
    "latest_date": "2026-11-07"
  }
}
```

#### `GET /api/v1/cotahist/tickers/{ticker}/quotes`

Get historical quotes for a ticker with filters.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `desde` | date | — | Start date (YYYY-MM-DD) |
| `ate` | date | — | End date (YYYY-MM-DD) |
| `dias` | u32 | — | Last N trading days |
| `campos` | string | all | Comma-separated field list |
| `ordem` | string | `desc` | `asc` or `desc` |
| `pagina` | u32 | 1 | Page number |
| `por_pagina` | u32 | 252 | Items per page (max 1000) |

**Response:**
```json
{
  "data": {
    "ticker": "VALE3",
    "quotes": [
      {
        "data": "2026-11-07",
        "pre_abr": 62.05,
        "pre_max": 62.89,
        "pre_min": 61.72,
        "pre_med": 62.31,
        "pre_ult": 62.17,
        "pre_ofe_compra": 62.15,
        "pre_ofe_venda": 62.19,
        "tot_neg": 245891,
        "quat_tot": 29847362,
        "vol_tot": 1847293440.0,
        "var_pct": 0.45,
        "var_abs": 0.28
      }
    ],
    "total": 5,
    "pagina": 1,
    "por_pagina": 252
  }
}
```

#### `GET /api/v1/cotahist/tickers/{ticker}/latest`

Get the latest quote for a ticker. Shortcut for `quotes?dias=1`.

**Response:**
```json
{
  "data": {
    "ticker": "VALE3",
    "data": "2026-11-07",
    "pre_abr": 62.05,
    "pre_max": 62.89,
    "pre_min": 61.72,
    "pre_med": 62.31,
    "pre_ult": 62.17,
    "pre_ofe_compra": 62.15,
    "pre_ofe_venda": 62.19,
    "tot_neg": 245891,
    "quat_tot": 29847362,
    "vol_tot": 1847293440.0,
    "var_pct": 0.45,
    "var_abs": 0.28
  }
}
```

#### `GET /api/v1/cotahist/dates/{date}/quotes`

Get all quotes for a specific date.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `pagina` | u32 | 1 | Page number |
| `por_pagina` | u32 | 100 | Items per page (max 500) |

**Response:**
```json
{
  "data": {
    "date": "2026-11-07",
    "quotes": [
      {
        "ticker": "VALE3",
        "pre_ult": 62.17,
        "var_pct": 0.45,
        "vol_tot": 1847293440.0
      }
    ],
    "total": 487,
    "pagina": 1,
    "por_pagina": 100
  }
}
```

### 7.4 IBOV Endpoints

#### `GET /api/v1/ibov/history`

Get IBOV historical series with filters.

**Query parameters:**

| Param | Type | Default | Description |
|---|---|---|---|
| `desde` | date | — | Start date |
| `ate` | date | — | End date |
| `dias` | u32 | — | Last N trading days |
| `ordem` | string | `desc` | `asc` or `desc` |
| `pagina` | u32 | 1 | Page number |
| `por_pagina` | u32 | 252 | Items per page (max 1000) |

**Response:**
```json
{
  "data": {
    "codigo": "IBOV",
    "nome": "IBOVESPA",
    "history": [
      {
        "data": "2026-11-07",
        "pre_abr": 108230.45,
        "pre_max": 108500.12,
        "pre_min": 107900.33,
        "pre_med": 108200.78,
        "pre_ult": 108450.67,
        "vol_tot": 15234567890.0,
        "num_neg": 450123,
        "qtd_tot": 1234567890.0
      }
    ],
    "total": 5,
    "pagina": 1,
    "por_pagina": 252
  }
}
```

#### `GET /api/v1/ibov/latest`

Get the latest IBOV data point.

**Response:**
```json
{
  "data": {
    "codigo": "IBOV",
    "nome": "IBOVESPA",
    "data": "2026-11-07",
    "pre_abr": 108230.45,
    "pre_max": 108500.12,
    "pre_min": 107900.33,
    "pre_med": 108200.78,
    "pre_ult": 108450.67,
    "vol_tot": 15234567890.0,
    "num_neg": 450123,
    "qtd_tot": 1234567890.0,
    "var_pct": 0.23
  }
}
```

### 7.5 System Endpoints

#### `GET /api/v1/health`

Health check for load balancers.

**Response:**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime_seconds": 86400,
  "data_files_loaded": 3,
  "cache_entries": 1247
}
```

#### `GET /api/v1/metrics`

Prometheus metrics endpoint. Returns text/plain in Prometheus exposition format.

#### `GET /api/v1/info`

Service information.

**Response:**
```json
{
  "name": "tucano-b3",
  "version": "1.0.0",
  "data_range": {
    "cotahist": { "min": "2000-01-03", "max": "2026-11-07" },
    "ibov": { "min": "1994-01-03", "max": "2026-11-07" }
  },
  "tickers_count": 1247,
  "total_quotes": 14231887,
  "cache": {
    "entries": 1247,
    "max_entries": 10000,
    "ttl_seconds": 300
  }
}
```

---

## 8. Data Model

### 8.1 Parquet Schema (from `tucano-b3-cli`)

The API reads from the exact same Parquet files produced by `tucano-b3-cli`. Schema is defined in PRODUCT.md of `tucano-b3-cli`.

### 8.2 API Response Models (Rust structs)

```rust
// src/models/quote.rs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QuoteResponse {
    pub ticker: String,
    pub data: String,           // YYYY-MM-DD
    pub pre_abr: f64,
    pub pre_max: f64,
    pub pre_min: f64,
    pub pre_med: f64,
    pub pre_ult: f64,
    pub pre_ofe_compra: Option<f64>,
    pub pre_ofe_venda: Option<f64>,
    pub tot_neg: u32,
    pub quat_tot: u64,
    pub vol_tot: f64,
    pub var_pct: Option<f64>,
    pub var_abs: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TickerInfo {
    pub ticker: String,
    pub isin: Option<String>,
    pub company_name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub is_active: bool,
    pub total_trading_days: u32,
    pub latest_price: Option<f64>,
    pub latest_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub pagina: u32,
    pub por_pagina: u32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub data: T,
    pub meta: ResponseMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ResponseMeta {
    pub request_id: String,
    pub timestamp: String,
    pub cached: bool,
    pub query_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiError {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub request_id: String,
}
```

---

## 9. Caching Strategy

### 9.1 Cache Design

| Aspect | Design |
|---|---|
| **Implementation** | `DashMap<String, CacheEntry>` (concurrent, lock-free reads) |
| **Key** | SHA256 hash of (endpoint + query params) |
| **Value** | Serialized JSON response bytes |
| **TTL** | Configurable, default 300 seconds |
| **Max entries** | Configurable, default 10,000 |
| **Eviction** | LRU when max entries reached |
| **Invalidation** | TTL-based + explicit purge endpoint (admin) |

### 9.2 Cache Key Generation

```rust
fn cache_key(method: &str, path: &str, query: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(method.as_bytes());
    hasher.update(path.as_bytes());
    hasher.update(query.as_bytes());
    hasher.finalize().to_hex().to_string()
}
```

### 9.3 Cache Configuration

```toml
# config.toml
[cache]
enabled = true
ttl_seconds = 300
max_entries = 10000
purge_on_data_reload = true
```

---

## 10. Rate Limiting

### 10.1 Design

| Aspect | Design |
|---|---|
| **Algorithm** | Token bucket |
| **Key** | API key (from `X-API-Key` header) or IP address |
| **Default limit** | 100 requests/minute per IP |
| **API key limit** | 1000 requests/minute per key |
| **Response on limit** | HTTP 429 with `Retry-After` header |

### 10.2 Configuration

```toml
# config.toml
[rate_limit]
enabled = true
default_requests_per_minute = 100
api_key_requests_per_minute = 1000
```

### 10.3 API Key Management

For v1, API keys are stored in a static TOML file:

```toml
# api_keys.toml
[[keys]]
key = "tucano-abc123..."
name = "Quant Research Team"
rate_limit = 1000

[[keys]]
key = "tucano-def456..."
name = "Portfolio Dashboard"
rate_limit = 500
```

---

## 11. Performance Targets

| Metric | Target | Measurement |
|---|---|---|
| **Throughput** | > 10,000 req/s (4-core, cached) | `wrk` or `vegeta` benchmark |
| **Latency p50** | < 5 ms (cached) | Prometheus histogram |
| **Latency p99** | < 50 ms (cached) | Prometheus histogram |
| **Latency p99** | < 100 ms (cold query) | Prometheus histogram |
| **Memory** | < 512 MB steady state | RSS monitoring |
| **Startup time** | < 2 seconds | Time to first successful request |
| **Binary size** | < 30 MB (stripped, LTO) | `ls -lh` |
| **Data load time** | < 5 seconds (all Parquet files) | Startup log |

---

## 12. Configuration

### 12.1 Config File (`config.toml`)

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4                    # tokio worker threads (0 = auto)

[data]
dir = "./data"                 # Path to data/ directory
reload_interval_seconds = 0    # 0 = no auto-reload

[cache]
enabled = true
ttl_seconds = 300
max_entries = 10000

[rate_limit]
enabled = true
default_requests_per_minute = 100
api_key_requests_per_minute = 1000

[logging]
level = "info"                 # trace, debug, info, warn, error
format = "json"                # json or pretty

[metrics]
enabled = true

[api_keys]
file = "./api_keys.toml"
```

### 12.2 Environment Variables

All config values can be overridden via environment variables:

```
TUCANO_SERVER_PORT=9090
TUCANO_DATA_DIR=/mnt/data
TUCANO_CACHE_TTL=600
TUCANO_LOG_LEVEL=debug
```

---

## 13. Observability

### 13.1 Structured Logging

All logs are JSON-formatted via `tracing-subscriber`:

```json
{
  "timestamp": "2026-11-10T14:32:01.123-03:00",
  "level": "INFO",
  "message": "Request completed",
  "request_id": "uuid-v4",
  "method": "GET",
  "path": "/api/v1/cotahist/tickers/VALE3/latest",
  "status": 200,
  "duration_ms": 0.42,
  "cached": true,
  "client_ip": "192.168.1.100"
}
```

### 13.2 Prometheus Metrics

```
# Request count
tucano_http_requests_total{method="GET", path="/api/v1/cotahist/tickers/{ticker}/latest", status="200"} 12345

# Request duration histogram
tucano_http_request_duration_seconds_bucket{method="GET", path="...", le="0.005"} 10000
tucano_http_request_duration_seconds_bucket{method="GET", path="...", le="0.01"} 11000
tucano_http_request_duration_seconds_bucket{method="GET", path="...", le="0.05"} 12000
tucano_http_request_duration_seconds_bucket{method="GET", path="...", le="0.1"} 12300
tucano_http_request_duration_seconds_bucket{method="GET", path="...", le="+Inf"} 12345

# Cache metrics
tucano_cache_hits_total 98765
tucano_cache_misses_total 24680
tucano_cache_entries 1247

# Rate limit metrics
tucano_rate_limited_total 42

# Data metrics
tucano_data_files_loaded 3
tucano_data_total_quotes 14231887
```

---

## 14. Security

### 14.1 Threat Model

| Threat | Mitigation |
|---|---|
| SQL injection | DataFusion parameterized queries; no raw SQL from user input |
| Path traversal | Data directory is fixed; no user-supplied file paths |
| Denial of service | Rate limiting; request size limits; timeout |
| Information disclosure | Error messages don't leak internal paths or stack traces |
| Header injection | All user inputs validated before use in headers |

### 14.2 Security Headers

All responses include:

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
Content-Security-Policy: default-src 'none'
```

### 14.3 Request Validation

- Query params validated before use
- Date params validated as ISO 8601
- Pagination params bounded (max 1000 per page)
- Request body rejected (GET-only API)

---

## 15. Deployment

### 15.1 Docker

```dockerfile
FROM rust:1.82-bookworm as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/tucano-b3 /usr/local/bin/tucano-b3
COPY config.toml /etc/tucano/config.toml

EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
  CMD curl -f http://localhost:8080/api/v1/health || exit 1

ENTRYPOINT ["tucano-b3"]
CMD ["--config", "/etc/tucano/config.toml"]
```

### 15.2 Docker Compose

```yaml
version: '3.8'

services:
  api:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./data:/data:ro
      - ./config.toml:/etc/tucano/config.toml:ro
    environment:
      - TUCANO_LOG_LEVEL=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/api/v1/health"]
      interval: 30s
      timeout: 3s
      retries: 3
```

### 15.3 Systemd Service

```ini
[Unit]
Description=tucano-b3 REST API
After=network.target

[Service]
Type=simple
User=tucano
ExecStart=/usr/local/bin/tucano-b3 --config /etc/tucano/config.toml
Restart=on-failure
RestartSec=5
Environment=TUCANO_LOG_LEVEL=info

[Install]
WantedBy=multi-user.target
```

---

## 16. Language Policy

Same as `tucano-b3-cli`:

| Layer | Language |
|---|---|
| API error messages | Brazilian Portuguese |
| API documentation (Swagger UI) | Brazilian Portuguese |
| Code identifiers | English |
| Code comments (LLM-facing) | English |
| Internal logs | English |
| Config file keys | English |
| README.md (user) | Brazilian Portuguese |
| README.dev.md (contributor) | English |
| OpenAPI descriptions | Brazilian Portuguese |

---

## 17. Milestones

| # | Milestone | Scope | Duration |
|---|---|---|---|
| M0 | Project skeleton | axum server, config, health endpoint, Docker | 1 week |
| M1 | Data layer | Parquet reader, DataFusion integration, schema validation | 2 weeks |
| M2 | COTAHIST endpoints | All 5 COTAHIST endpoints with pagination | 2 weeks |
| M3 | IBOV endpoints | Both IBOV endpoints with pagination | 1 week |
| M4 | Caching | DashMap cache, TTL, LRU eviction, cache metrics | 1 week |
| M5 | Rate limiting + API keys | Token bucket, API key file, 429 responses | 1 week |
| M6 | OpenAPI + Swagger | utoipa annotations, Swagger UI endpoint | 1 week |
| M7 | Observability | Prometheus metrics, structured logging, request tracing | 1 week |
| M8 | Performance optimization | Benchmarking, profiling, connection pooling, query optimization | 1 week |
| M9 | Packaging + release | CI/CD, Docker, release binaries, documentation | 1 week |

**Total: ~12 weeks to 1.0.**

---

## 18. Testing Strategy

### 18.1 Coverage Mandate

**100% unit test coverage** on business logic (services, models, cache, rate limiter).

**Exclusions:**
- Generated code (OpenAPI schemas)
- `main.rs` entry point
- Integration test helpers

### 18.2 Testing Levels

| Level | Location | Scope |
|---|---|---|
| **Unit** | Inline `#[cfg(test)]` | Services, models, cache logic, rate limiter |
| **Integration** | `tests/` | Full HTTP request/response cycles |
| **API contract** | `tests/api/` | OpenAPI spec validation |
| **Load** | `benches/` | Throughput and latency benchmarks |
| **Property** | `proptest` | Date parsing, pagination edge cases |

### 18.3 Integration Test Pattern

```rust
// tests/api/cotahist.rs

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn test_get_ticker_latest() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/VALE3/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["ticker"], "VALE3");
    assert!(json["data"]["pre_ult"].is_number());
}

#[tokio::test]
async fn test_get_unknown_ticker_returns_404() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cotahist/tickers/XYZXYZ/latest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "TICKER_NOT_FOUND");
    assert!(json["error"]["message"].as_str().unwrap().contains("XYZXYZ"));
}
```

---

## 19. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| DataFusion query latency too high for cold queries | High | Pre-warm cache on startup; materialize hot queries |
| Memory exhaustion from large cache | Medium | Hard cap on cache entries; LRU eviction; monitor RSS |
| Parquet schema changes break API | High | Schema version check on startup; fail-fast if mismatch |
| Rate limiter bypass via IP spoofing | Medium | Use X-Forwarded-For only behind trusted proxy |
| API becomes bottleneck under high load | High | Horizontal scaling; stateless design; load balancer |
| Portuguese error messages drift | Low | Locale test suite; CI check |
| OpenAPI spec drifts from implementation | Medium | Generate spec from code (utoipa); snapshot tests |
| DataFusion memory usage on complex queries | Medium | Query timeout; result size limit; streaming responses |

---

## 20. Success Metrics (90 days post-1.0)

| Metric | Target |
|---|---|
| Throughput | > 10,000 req/s sustained |
| Latency p99 | < 50 ms (cached) |
| Latency p99 | < 100 ms (cold) |
| Uptime | > 99.9% |
| Cache hit rate | > 80% |
| Error rate | < 0.1% |
| API consumers | ≥ 5 external users |
| GitHub stars | ≥ 50 |

---

## 21. Open Questions for Architecture Review

1. Should `tucano-b3` support hot-reload of Parquet files (watch for changes) or require restart?
2. Should we add a `/api/v1/admin/reload` endpoint for triggering data reload without restart?
3. Should the API support response compression (zstd/gzip) for large result sets?
4. Should we add WebSocket support in v1.1 for real-time ticker streaming?
5. Should the API key system integrate with an external identity provider in v2?

---

## 22. Appendix: Portuguese Glossary

| Portuguese (API) | English (code) |
|---|---|
| `pagina` | `page` |
| `por_pagina` | `per_page` |
| `busca` | `search` |
| `ativo` | `active` |
| `desde` | `since` |
| `ate` | `until` |
| `dias` | `days` |
| `campos` | `fields` |
| `ordem` | `order` |
| `cotações` | `quotes` |
| `ticker não encontrado` | `ticker not found` |
| `limite de requisições excedido` | `rate limit exceeded` |
| `erro interno do servidor` | `internal server error` |
| `serviço indisponível` | `service unavailable` |
| `nenhum dado encontrado` | `no data found` |
