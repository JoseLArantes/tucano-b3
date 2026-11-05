# PRD: B3-Only Financial Data Normalization API (`tucano-b3` — Rust Edition)

## 1. Product Summary

Build a high-efficiency, ultra-low-cost **Rust** application (`tucano-b3`) inspired by the engineering principles of `tucano-cvm` that downloads, parses, normalizes, stores, and exposes **only official B3 public financial market data**. The product focuses strictly on Brazilian exchange data from B3 public bulletins, reference tables, index portfolios, and historical quotations—with **zero enrichment** from CVM, Yahoo Finance, Fundamentus, StatusInvest, broker feeds, paid third-party APIs, or unofficial mirrors.

By building `tucano-b3` in **Rust** (`axum` + `sqlx` + `quick-xml` + `rust_decimal` + `utoipa` + `tokio`), the platform achieves:
- **Ultra-Low API Call Cost & Footprint**: A single ~15–25 MB statically linked binary idling at **<25 MB RAM** and serving **50,000+ req/s per vCPU**, eliminating the need for heavy runtime interpreters or external Redis/Celery broker infrastructure.
- **Zero-Copy Streaming Ingestion**: Processes 500MB+ ISO 20022 B3 XML files (`BVBG.028.02`, `BVBG.086.01`) in <2 seconds with **<30 MB peak RAM** via `quick-xml` pull parsing, and slices 1,000,000+ fixed-width 245-byte `COTAHIST` lines in <1 second via zero-allocation `[u8; 245]` byte slicing and stack-allocated 128-bit `rust_decimal::Decimal`.
- **PostgreSQL-Native Background Queue & Locking**: Runs the HTTP API, Read-Only MCP server, scheduled cron runner, and background ingestion worker either inside a single unified binary or as separate container entrypoints (`tucano-b3 serve`, `tucano-b3 worker`, `tucano-b3 mcp`) backed directly by PostgreSQL (`SKIP LOCKED` + `pg_try_advisory_lock`)—removing Redis and Celery operational overhead entirely.

The application upholds the five core engineering pillars established in `tucano-cvm`:
1. **Primary Sources Only**: Every dataset is traced to an immutable, SHA-256-verified official B3 file or payload (`BVBG.028.02`, `BVBG.086.01`, `BVBG.186.01`, `BVBG.087.01`, `Tradable Security List`, B3 index portfolio composition sources, and `COTAHIST`).
2. **Idempotency**: Re-ingesting the same file or re-running a failed sync job (with `force_reparse=false` or `force_reparse=true`) never produces duplicate records, phantom updates, or `UniqueViolation` errors. Nullable columns in composite natural keys are protected with PostgreSQL 15+ `UNIQUE NULLS NOT DISTINCT`.
3. **Consistency**: The B3 instrument registry (`BVBG.028.02` / `InstrumentsConsolidated`) is the canonical root identity (`b3_instruments`). Temporal identifier validity windows (`valid_from` / `valid_to`) are closed deterministically when instruments change tickers or expire, and corporate distribution revisions in `COTAHIST` (`DISMES` / `distribution_number`) are part of the natural key.
4. **Efficiency**: Two-phase ingestion separates file discovery/download/member-mapping from batched PostgreSQL `UNNEST(...)` set-based upserts (`INSERT ... ON CONFLICT DO UPDATE WHERE ... IS DISTINCT FROM`), streaming `quick-xml` parsing, and zero-allocation fixed-width parsing.
5. **Determinism**: Compile-time type safety, 128-bit fixed-point `Decimal` arithmetic (never `f64`), timezone-aware UTC timestamps (`chrono::DateTime<Utc>`), B3 market calendar awareness, deterministic tie-breaker sorting on all queries, and exact ISO 20022 XML path resolution guarantee identical outputs given identical inputs.

Public endpoints must be grouped by source file family, not by generic business concept, so consumers can distinguish data originating from `BVBG.028.02`, `BVBG.086.01`, `BVBG.186.01`, `BVBG.087.01`, `Tradable Security List`, B3 index portfolio composition sources, and `COTAHIST` (`Yearly`, `Monthly`, `Daily`). Identifiers such as `CNPJ_CIA` and `CD_CVM` must not be used as anchors in this product because they are outside the allowed B3-only source scope.

---

## 2. Rust Architecture & Design Critique (Lessons from `tucano-cvm` & B3 Domain Research)

### 2.1 Why Rust + PostgreSQL-Only Replaces Python + FastAPI + Celery + Redis
| Dimension | Legacy Python (`FastAPI` + `SQLAlchemy` + `Celery` + `Redis`) | Target Rust Stack (`Axum` + `SQLx` + `quick-xml` + `Tokio`) |
| :--- | :--- | :--- |
| **Idle Memory Footprint** | ~400–600 MB across API, Celery Worker, Beat, and Redis | **~15–30 MB** (No Redis required; Tokio worker + scheduler built-in) |
| **500MB XML Parse (`BVBG.028` / `086`)** | 2.5 GB+ RAM (`ElementTree.fromstring` OOM risk), 30–60s | **<25 MB RAM** (`quick-xml` streaming pull parser), ~1.5s |
| **1M Rows `COTAHIST` Parse + SHA-256** | ~45–90s (heap-allocated strings, `Decimal` objects, dicts) | **~0.5s** (`&[u8; 245]` stack slicing + `sha2` crate + `rust_decimal`) |
| **Database Upsert Throughput** | Row-by-row ORM `session.scalar()` + `flush()` (~2M queries/yr file) | **Batched `UNNEST` set-based SQL** (5,000 rows/query + CTE audit trigger) |
| **API Latency & Cost** | ~1,500–2,500 req/s per vCPU | **~50,000+ req/s per vCPU** (`serde` + `axum` zero-overhead routing) |
| **Container Image Size** | ~400–700 MB (`python:3.12-slim` + C wheels) | **~20–35 MB** (`debian:bookworm-slim` or `distroless/cc` static binary) |

### 2.2 Primary Sources & Acquisition Reality
- **Official B3 Host Topology**: B3 distributes public files across multiple official domains (`www.b3.com.br`, `bvmf.bmfbovespa.com.br`, `arquivos.b3.com.br`, and `sistemaswebb3-listados.b3.com.br`). `B3_ALLOWED_HOSTS` defaults to all four official B3 domains.
- **Two-Step Tokenized Downloads (`arquivos.b3.com.br`)**: Modern B3 BDI/UP2DATA public endpoints use a 2-step handshake (`/api/download/requestname?fileName=...&date=...` returning a temporary token, followed by `/api/download/?token=...`). The `reqwest` download service natively supports both direct GET (`pesquisapregao/download?filelist=...`) and 2-step B3 token handshakes.
- **Multi-Member Nested ZIPs**: B3 `pesquisapregao` archives (`PR{yymmdd}.zip`, `IN{yymmdd}.zip`, `IR{yymmdd}.zip`, `SPRE{yymmdd}.zip`) frequently contain nested `.zip` archives and multiple XML split parts (`BVBG.086.01` part 1, part 2, or intraday/after-market revisions). Using the `zip` crate, the extractor recursively inventories and streams **all** valid member files in deterministic sequence (`b3_source_file_members`).
- **COTAHIST Granularities**: Supports `COTAHIST_A{YYYY}.ZIP` (yearly), `COTAHIST_M{MMYYYY}.ZIP` (monthly), and `COTAHIST_D{DDMMYYYY}.ZIP` (daily) using the exact same 245-byte parser (`encoding_rs::WINDOWS_1252` / `ISO-8859-1` decoding).
- **Detection-First Scanner (`Updates Service`)**: Following `tucano-cvm`, lightweight source discovery (checking HTTP `HEAD`, `ETag`, `Last-Modified`, or B3 channel publication metadata into `b3_pending_updates`) is separated from heavy download and ingestion.

### 2.3 Idempotency & Natural Key Integrity
- **PostgreSQL Nullable Composite Unique Constraints**: In PostgreSQL, standard `UNIQUE (col_a, col_b, col_c)` constraints treat `NULL != NULL`. All composite unique constraints with nullable columns (`security_id`, `market_code`, `instrument_id`, `days_to_settlement`) use PostgreSQL 15+ `UNIQUE NULLS NOT DISTINCT (...)`.
- **COTAHIST Natural Key (`distribution_number` / `DISMES`)**: Positions 243–245 (`DISMES` - Número de Distribuição) distinguish quotes of the same ticker on the same trading session across different corporate action distributions. The natural key is `(trade_date, ticker_symbol, market_type, bdi_code, distribution_number)`.
- **PriceReport Natural Key (`days_to_settlement` / `DaysToSttlm`)**: Forward (*termo*) contracts in `BVBG.086.01` for the same security/ticker on the same `trade_date` are differentiated by `days_to_settlement`. The natural key is `(trade_date, security_id, ticker_symbol, market_identifier_code, days_to_settlement)`.
- **Three-Timestamp Contract (`created_at`, `last_seen_at`, `data_changed_at`)**: Mirroring `tucano-cvm`'s `criado_em`, `sincronizado_em`, and `alterado_em`, every normalized table tracks `created_at`, `last_seen_at`, `data_changed_at` (updated **only** when a business attribute changes), and `field_changed_at` (JSONB map of per-field change timestamps), plus row-level provenance (`source_line_number`, `source_member_name`).
- **Active Writing to `b3_field_changes` and `b3_quarantine_records`**: Every field mutation emits a `b3_field_changes` audit row, and any malformed/unparseable record is persisted in `b3_quarantine_records` (like `registros_quarentena` in `tucano-cvm`) with replay support rather than aborting the batch.

### 2.4 Consistency & Temporal Resolution
- **Closing Temporal Identifier Windows (`valid_to`)**: `b3_instrument_identifiers` reuses open identifier windows (`valid_to IS NULL`) when unchanged on daily `BVBG.028.02` runs, and automatically closes them (`valid_to = previous_reference_date`) only when an instrument's ticker/ISIN changes or the instrument is delisted.
- **Retroactive Late-Binding Resolution (`Rebuild Identity`)**: When `COTAHIST` historical years are backfilled before or outside the window of an instrument file, rows with `resolution_status = 'unresolved'` are re-evaluatable via an idempotent **Identity Resolution Rebuild Job** (`POST /api/v1/b3/sync-runs/rebuild-identity`) that links `instrument_id` in bulk SQL without mutating market business fields.

---

## 3. Rust Technology Stack & Crate Architecture

### 3.1 Core Rust Crates (`Cargo.toml`)
- **Language & Toolchain**: Rust `1.82+` (`edition = "2021"`, `tokio` async runtime).
- **HTTP API & Routing**: `axum` (`0.7+` / `0.8+`), `tower`, `tower-http` (`cors`, `trace`, `compression-gzip`, `limit`).
- **OpenAPI & Schema Generation**: `utoipa` + `utoipa-swagger-ui` + `utoipa-redoc` (compile-time macro-driven OpenAPI 3.1 generation serving `/docs`, `/redoc`, and `/openapi.json`).
- **Database & Migrations**: `sqlx` (`0.8+` with `postgres`, `runtime-tokio-rustls`, `chrono`, `rust_decimal`, `uuid`, `json` features) + `sqlx::migrate!("./migrations")` embedded compile-time/startup migrations.
- **Serialization & Canonical Hashing**: `serde`, `serde_json` (with `preserve_order` / deterministic BTreeMap canonical JSON serialization), `sha2` (`Sha256`), `hex`.
- **Precision Financial Math & Time**: `rust_decimal` (128-bit fixed-point decimal, zero float drift), `chrono` (`NaiveDate`, `DateTime<Utc>`), `chrono-tz` (`America::Sao_Paulo`).
- **High-Speed Streaming Parsers**:
  - XML (`BVBG.028.02`, `BVBG.086.01`, `BVBG.186.01`, `BVBG.087.01`): `quick-xml` (zero-copy streaming pull parser `Reader::from_reader`).
  - CSV (`Tradable Security List`, `Index Portfolio Composition`, `InstrumentsConsolidated`): `csv` crate with flexible delimiter detection (`;`, `,`, `|`, `\t`).
  - Fixed-Width (`COTAHIST`): Custom zero-allocation `[u8; 245]` slice parser + `encoding_rs` (`WINDOWS_1252` / `ISO-8859-1` to UTF-8).
  - Archive Extraction: `zip` crate (streaming multi-member & nested ZIP reader).
- **HTTP Client & Resilience**: `reqwest` (`rustls-tls`, `stream`, `gzip`) with exponential backoff retry policy.
- **Observability**: `tracing`, `tracing-subscriber` (structured JSON logs), `metrics`, `metrics-exporter-prometheus` (`/metrics` endpoint).
- **Scheduler & Background Worker**: `tokio-cron-scheduler` (or Tokio interval loop with timezone awareness) + PostgreSQL advisory locks (`pg_try_advisory_lock`) and `b3_sync_runs` state machine.
- **Testing & Quality**: Built-in `cargo test` unit tests, `sqlx::test` database integration tests, `wiremock` (for mocking B3 HTTP download servers), `clippy` (`-D warnings`), and `rustfmt`.

### 3.2 Workspace / Module Layout
```text
tucano-b3/
  Cargo.toml
  Cargo.lock
  Dockerfile
  docker-compose.yml
  .env.example
  migrations/
    0001_initial_b3_schema.sql
  src/
    main.rs                 # CLI entrypoint: `serve`, `worker`, `migrate`, `mcp`
    lib.rs                  # Library root exported for unit/integration tests
    config.rs               # Environment configuration (B3Settings)
    error.rs                # Unified AppError -> HTTP status & OpenAPI error schemas
    telemetry.rs            # tracing + Prometheus /metrics setup
    auth.rs                 # Bearer token middleware (TUCANO_B3_TOKEN)
    db/
      mod.rs                # PgPool initialization & healthcheck
      models.rs             # Domain & operational structs
      enums.rs              # SourceFileType, ResolutionStatus, SyncRunStatus, etc.
      locks.rs              # PostgreSQL advisory locks (pg_try_advisory_lock)
      upsert.rs             # Batched changed-field-only upsert + b3_field_changes audit
      resolver.rs           # In-memory & bulk SQL RelationshipResolver + rebuild_identity
    parsers/
      mod.rs                # Unified trait & archive multi-member dispatcher
      canonical.rs          # Deterministic canonicalization, implied decimals, SHA-256 row hash
      xml_utils.rs          # quick-xml streaming helpers & ISO 20022 path extractors
      bvbg_028.rs           # BVBG.028.02 Instruments File streaming XML parser
      bvbg_086.rs           # BVBG.086.01 PriceReport streaming XML parser
      bvbg_186.rs           # BVBG.186.01 Simplified Equities streaming XML parser
      bvbg_087.rs           # BVBG.087.01 IndexReport (IndxInf, IOPVInf, BDRInf) parser
      cotahist.rs           # 245-byte fixed-width COTAHIST (Yearly/Monthly/Daily) parser
      tradable_list.rs      # Tradable Security List CSV parser
      index_composition.rs  # B3 Index Portfolio Composition CSV/JSON parser
    services/
      mod.rs
      download_service.rs   # Host-validated downloader (direct GET + 2-step B3 token)
      sync_service.rs       # Two-phase ingestion pipeline, quarantine & member tracking
      updates_scanner.rs    # Detection-first HTTP HEAD / channel scanner (b3_pending_updates)
      calendar_service.rs   # B3 trading calendar validation
    api/
      mod.rs                # Axum router + Utoipa OpenAPI 3.1 assembly
      schemas.rs            # Source-specific response DTOs, Provenance, Pagination
      handlers/
        health.rs           # /api/v1/health, /api/v1/b3/worker/health
        instruments.rs      # /api/v1/b3/instruments/*
        tradable.rs         # /api/v1/b3/tradable-securities/*
        price_report.rs     # /api/v1/b3/price-report/* & /api/v1/b3/price-report-comparison/*
        simplified_price.rs # /api/v1/b3/equities-simplified-price-report/*
        index_report.rs     # /api/v1/b3/index-report/*
        index_comp.rs       # /api/v1/b3/index-composition/*
        cotahist.rs         # /api/v1/b3/cotahist/*
        governance.rs       # /api/v1/b3/source-files, sync-runs, quarantine, field-changes, diagnostics
    worker/
      mod.rs                # Background job runner & cron scheduler
    mcp/
      mod.rs                # Read-only Analytical MCP server (stdio & /mcp JSON-RPC)
  tests/
    parser_tests.rs
    canonical_and_hash_tests.rs
    upsert_and_audit_tests.rs
    resolver_and_rebuild_tests.rs
    sync_service_tests.rs
    api_contract_tests.rs
```

---

## 4. Source Research Basis

Official B3 references used to define scope:
- **B3 Search by Trading Session (`Pesquisa por Pregão`)**: Lists daily downloadable files including `BVBG.028.02 Instruments File` (`IN{yymmdd}.zip`), `BVBG.086.01 PriceReport` (`PR{yymmdd}.zip`), `BVBG.087.01 IndexReport` (`IR{yymmdd}.zip`), `BVBG.186.01 - Simplified Price Report - Equities` (`SPRE{yymmdd}.zip`), and `Tradable Security List` (`SecurityList{yymmdd}.zip`).
- **B3 BDI / UP2DATA Public File Portal (`arquivos.b3.com.br`)**: Hosts daily consolidated public tables such as `InstrumentsConsolidated` via tokenized download APIs.
- **B3 InstrumentReport Catalog (`BVBG.028.02`)**: Defines the ISO 20022 XML structure (`Instrm`, `FinInstrmId`, `FinInstrmAttrCmon`, `InstrmInf`, `EqtyInf`, `OptnOnEqtsInf`, `FutrCtrctInf`, etc.).
- **B3 Prices and Indices Catalog (`BVBG.086.01`, `BVBG.087.01`, `BVBG.186.01`)**: Documents `BVBG.086.01 PriceReport` (`PricRpt`), `BVBG.186.01 EquitiesSimplifiedPriceReport`, and `BVBG.087.01 IndexReport` (`IndxRpt` / `IndxInf` / `IOPVInf` / `BDRInf`).
- **B3 COTAHIST Documentation (`SeriesHistoricas_Layout.pdf`)**: Defines `COTAHIST_A{AAAA}.ZIP` (annual), `COTAHIST_M{MMAAAA}.ZIP` (monthly), and `COTAHIST_D{DDMMAAAA}.ZIP` (daily) fixed-width 245-byte historical quotation files.
- **B3 Index Composition Portal (`sistemaswebb3-listados.b3.com.br`)**: Official B3 Listed Portal endpoints (`GetPortfolioDay` / `DownloadPortfolioDay`) publishing official theoretical quantities and participation weights for B3 indices.

Reference links:
- B3 Search by Trading Session: https://www.b3.com.br/en_us/market-data-and-indices/data-services/market-data/historical-data/newsletters/search-by-trading-session/
- B3 InstrumentReport catalog (`BVBG.028.02`): https://www.b3.com.br/data/files/0B/A1/CA/73/86072710547B5127AC094EA8/Catalogo-de-Mensagens-Cadastro-de-Instrumento-Versao-2.6.pdf
- B3 prices and indices catalog (`BVBG.086.01`, `BVBG.087.01`, `BVBG.186.01`): https://www.b3.com.br/data/files/81/91/9A/17/27B5C710BD0885C7AC094EA8/Catalogo_precos_v1.2.1.pdf
- B3 COTAHIST file layout: https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf

---

## 5. Goals & Non-Goals

### 5.1 Goals
- Expose official B3 public market data through a documented Rust `axum` REST API and a Read-Only Analytical MCP server.
- Achieve minimal cloud compute cost (<30 MB idle RAM, single static binary image, zero Redis/Celery dependency).
- Enforce B3-only provenance at code, configuration, network, API, and database levels.
- Normalize raw B3 files into relational tables with stable keys, `NULLS NOT DISTINCT` uniqueness guarantees, source provenance (down to archive member and line/record index), and clear field lineage.
- Preserve file-specific API surfaces so consumers can choose the exact B3 source they trust without silent cross-source merging.
- Execute two-phase, chunked, idempotent ingestion with `force_reparse` atomic cleanup, quarantine isolation (`b3_quarantine_records`), and retroactive identity resolution (`rebuild_identity`).
- Update stored field values only when the normalized canonical business value changes, recording field mutations in both `field_changed_at` JSONB and `b3_field_changes` audit rows.
- Ship with comprehensive Rust unit and integration tests (`cargo test`) and a complete, production-grade `docker-compose.yml`.

### 5.2 Non-Goals
- No Python runtime, FastAPI, SQLAlchemy, Celery, or Redis dependencies in the target runtime stack.
- No CVM fundamental data (`DFP`, `ITR`, `FRE`, `FCA`) and no `CNPJ_CIA` or `CD_CVM` relationship joins.
- No calculated adjusted prices (dividend adjustments, split adjustments, inflation adjustments, or total return calculations) unless those exact values are directly published in an allowed B3 source file.
- No real-time/intraday tick quotes or order book feeds.
- No scraping of third-party finance websites (Yahoo Finance, yfinance, Alpha Vantage, StatusInvest, Fundamentus, Investing.com, Wikipedia).

---

## 6. Data Source Scope & Technical Layouts

### 6.1 `BVBG.028.02 Instruments File` (and `InstrumentsConsolidated`)
**Purpose:**
- Canonical B3 instrument registry (`b3_instruments` & `b3_instrument_identifiers`).
- Must be the first normalized dataset ingested in any environment.
- Creates or updates canonical instrument identities and manages temporal validity windows (`valid_from` / `valid_to`) for tickers, ISINs, and aliases.

**Expected Format & Acquisition:**
- Primary: B3 ISO 20022 XML message file inside `IN{yymmdd}.zip` (`https://www.b3.com.br/pesquisapregao/download?filelist=IN{yymmdd}.zip`), schema `urn:bvmf.100.02.xsd` (`BVMF.100.02` / `BVBG.028.02`).
- Secondary/Fallback official B3 public registry: `InstrumentsConsolidated` CSV/ZIP from `arquivos.b3.com.br` tokenized download API.
- Parsed using **streaming `quick_xml::Reader`** over `<Instrm>` (and legacy fixture `<Instrument>`) blocks.

**Official B3 XML Path Mapping (`BVBG.028.02`):**
- `security_id`: `FinInstrmId/OthrId/Id` (B3 Trade Structure sequential instrument ID)
- `security_source`: `FinInstrmId/OthrId/Tp/Prtry` (e.g., `8`)
- `market_identifier_code`: `FinInstrmId/PlcOfListg/MktIdrCd` (default `BVMF`)
- `reference_date`: `RptParams/RptDtAndTm/Dt`
- `asset`: `FinInstrmAttrCmon/Asst`
- `asset_description`: `FinInstrmAttrCmon/AsstDesc`
- `market_code`: `FinInstrmAttrCmon/Mkt`
- `market_name`: `FinInstrmAttrCmon/MktDesc`
- `segment_code`: `FinInstrmAttrCmon/Sgmt`
- `segment_name`: `FinInstrmAttrCmon/SgmtDesc`
- **Instrument Subtype Block** (`InstrmInf/EqtyInf`, `InstrmInf/OptnOnEqtsInf`, `InstrmInf/FutrCtrctInf`, `InstrmInf/OptnOnSpotAndFutrsInf`, `InstrmInf/DrvtvConfdInf`, `InstrmInf/AdtlInf`):
  - `instrument_type`: XML subtype tag name (e.g., `EqtyInf`, `OptnOnEqtsInf`, `FutrCtrctInf`)
  - `ticker_symbol`: `InstrmInf/*/TckrSymb` or `FinInstrmAttrCmon/TckrSymb`
  - `isin`: `InstrmInf/*/ISIN` or `FinInstrmAttrCmon/ISIN`
  - `security_category`: `InstrmInf/*/SctyCtgy`
  - `cfi_code`: `InstrmInf/*/CFICd`
  - `instrument_status`: `InstrmInf/*/InstrmSts` or `FinInstrmAttrCmon/InstrmSts`
  - `maturity_date` / `expiration_date`: `InstrmInf/*/XprtnDt` or `InstrmInf/*/MtrtyDt`

**Temporal Identifier Window Rules (`b3_instrument_identifiers`):**
- When `BVBG.028.02` is ingested for `reference_date`:
  1. If an active identifier window (`valid_to IS NULL`) already exists for `(instrument_id, identifier_type, identifier_value, market_code)`, **do not insert a new row**; keep `valid_from` as the earliest observed date and update `last_seen_at`.
  2. If an instrument changes its `ticker_symbol` or `isin` on `reference_date`, close the previous active window by setting `valid_to = reference_date - 1 day` and open a new window with `valid_from = reference_date` and `valid_to = NULL`.

---

### 6.2 `Tradable Security List`
**Purpose:**
- Daily list of instruments B3 identifies as tradable (`SecurityList{yymmdd}.zip` or BDI daily tradable table).
- Supplements the base registry with session-level tradability status and trading dates.

**Expected Relationship:**
- References `b3_instruments.id` via `security_id` (primary) or `ticker_symbol + market_code + reference_date` / `isin` (fallback).
- Natural key: `UNIQUE NULLS NOT DISTINCT (reference_date, security_id, ticker_symbol, market_code)`.
- Unresolved rows are stored in `b3_tradable_securities_staging` with `resolution_status = 'unresolved'` (or `'ambiguous'`) and can be promoted to `b3_tradable_securities` when the Identity Resolution Rebuild job runs.

---

### 6.3 `BVBG.086.01 PriceReport`
**Purpose:**
- Authoritative daily B3 EOD file (`PR{yymmdd}.zip`) for complete price, volume, open interest, and transaction metrics across all B3 markets.

**Expected Format & XML Mapping (`urn:bvmf.217.01.xsd`):**
- Streaming `quick_xml::Reader` over `<PricRpt>` (and legacy fixture `<PriceRecord>` / `<Quote>`) elements across **all** XML member files inside `PR{yymmdd}.zip`.
- Official XML Paths:
  - `trade_date`: `TradDt/Dt`
  - `ticker_symbol`: `SctyId/TckrSymb`
  - `security_id`: `FinInstrmId/OthrId/Id`
  - `market_identifier_code`: `FinInstrmId/PlcOfListg/MktIdrCd`
  - `days_to_settlement`: `TradDtls/DaysToSttlm`
  - `trade_quantity`: `FinInstrmAttrCmon/TradQty`
  - `financial_volume_brl`: `FinInstrmAttrCmon/NtlFinVol`
  - `financial_volume_usd`: `FinInstrmAttrCmon/IntlFinVol`
  - `open_interest`: `FinInstrmAttrCmon/OpnIntrst`
  - `financial_instrument_quantity`: `FinInstrmAttrCmon/FinInstrmQty`
  - `best_bid_price`: `FinInstrmAttrCmon/BestBidPric`
  - `best_ask_price`: `FinInstrmAttrCmon/BestAskPric`
  - `first_price`: `FinInstrmAttrCmon/FrstPric`
  - `minimum_price`: `FinInstrmAttrCmon/MinPric`
  - `average_price`: `FinInstrmAttrCmon/TradAvrgPric`
  - `maximum_price`: `FinInstrmAttrCmon/MaxPric`
  - `last_price`: `FinInstrmAttrCmon/LastPric`
  - `regular_transactions_quantity`: `FinInstrmAttrCmon/RglrTxsQty`
  - `non_regular_transactions_quantity`: `FinInstrmAttrCmon/NonRglrTxsQty`
  - `regular_traded_contracts`: `FinInstrmAttrCmon/RglrTraddCtrcts`
  - `non_regular_traded_contracts`: `FinInstrmAttrCmon/NonRglrTraddCtrcts`
  - `national_regular_volume`: `FinInstrmAttrCmon/NtlRglrVol`
  - `national_non_regular_volume`: `FinInstrmAttrCmon/NtlNonRglrVol`
  - `international_regular_volume`: `FinInstrmAttrCmon/IntlRglrVol`
  - `international_non_regular_volume`: `FinInstrmAttrCmon/IntlNonRglrVol`
  - `oscillation_percentage`: `FinInstrmAttrCmon/OscnPctg`
  - `maximum_trade_limit`: `FinInstrmAttrCmon/MaxTradLmt`
  - `minimum_trade_limit`: `FinInstrmAttrCmon/MinTradLmt`

**Natural Key:**
- `UNIQUE NULLS NOT DISTINCT (trade_date, security_id, ticker_symbol, market_identifier_code, days_to_settlement)`.

---

### 6.4 `BVBG.186.01 Simplified Price Report - Equities`
**Purpose:**
- Simplified equities price report (`SPRE{yymmdd}.zip`).
- Stored in `b3_equities_simplified_price_quotes` and exposed in its own namespace; never overwrites `BVBG.086.01`.

**Expected Relationship & Natural Key:**
- References `b3_instruments.id`.
- Natural key: `UNIQUE NULLS NOT DISTINCT (trade_date, security_id, ticker_symbol, market_identifier_code)`.

---

### 6.5 `BVBG.087.01 IndexReport`
**Purpose:**
- B3-published index values (`IndxInf`), ETF Indicative Optimized Portfolio Values (`IOPVInf`), and BDR Reference Prices (`BDRInf`) from `IR{yymmdd}.zip` (`urn:bvmf.218.01.xsd`).

**Official XML Mapping (`BVBG.087.01`):**
- `trade_date`: `TradDt/Dt`
- Supports `IndxInf`, `IOPVInf`, and `BDRInf` sections (plus simplified test fixture `<IndexValue>` tags), normalizing `index_code` (`SctyId/TckrSymb`), `index_name` (`SctyInf/SctyDesc`), `value_type` (`CLOSING`, `OPENING`, `MIN`, `MAX`, `IOPV`, `BDR_REF`, etc.), `value` (`ClsgPric`, `Val`, etc.), `currency`, and `market_identifier_code`.
- Natural Key: `UNIQUE NULLS NOT DISTINCT (trade_date, index_code, value_type)`.

---

### 6.6 B3 Index Portfolio Composition
**Purpose:**
- B3-published index basket/portfolio composition snapshots (`b3_index_portfolio_compositions`).

**Allowed Official Sources:**
- Official B3 Listed Portal endpoints (`https://sistemaswebb3-listados.b3.com.br/indexProxy/indexCall/GetPortfolioDay/{base64}` and `DownloadPortfolioDay`), or official B3 index composition CSV/XML bulletins.

**Natural Key & Fields:**
- `UNIQUE NULLS NOT DISTINCT (composition_date, index_code, ticker_symbol, instrument_id)`
- Fields: `composition_date`, `index_code`, `ticker_symbol`, `isin`, `asset_name`, `theoretical_quantity`, `portfolio_weight`, `participation_percentage`, `instrument_id`, `resolution_status`.

---

### 6.7 `COTAHIST` (`COTAHIST_A{AAAA}.ZIP`, `COTAHIST_M{MMAAAA}.ZIP`, `COTAHIST_D{DDMMAAAA}.ZIP`)
**Purpose:**
- Official consolidated historical quotation source for raw OHLCV, quotation factors, and option specifications (`https://bvmf.bmfbovespa.com.br/InstDados/SerHist/`).

**Expected Format (Zero-Allocation Rust Byte Slicing):**
- Fixed-width text file (`ISO-8859-1` / `WINDOWS-1252`), 245 bytes per line:
  - Record type `00` (bytes `0..2`): Header (`COTAHIST.AAAA`, origin `BOVESPA `, generation date).
  - Record type `01` (bytes `0..2`): Detail quotation record:
    - `trade_date` (`2..10`, `YYYYMMDD`)
    - `bdi_code` (`10..12`)
    - `ticker_symbol` (`12..24`)
    - `market_type` (`24..27`)
    - `issuer_short_name` (`27..39`)
    - `paper_specification` (`39..49`)
    - `term_days` (`49..52`)
    - `currency` (`52..56`)
    - `open_price` (`56..69`, implied scale 2)
    - `high_price` (`69..82`, implied scale 2)
    - `low_price` (`82..95`, implied scale 2)
    - `average_price` (`95..108`, implied scale 2)
    - `last_price` (`108..121`, implied scale 2)
    - `best_bid_price` (`121..134`, implied scale 2)
    - `best_ask_price` (`134..147`, implied scale 2)
    - `trades_count` (`147..152`, integer)
    - `traded_quantity` (`152..170`, implied scale 0)
    - `traded_volume` (`170..188`, implied scale 2)
    - `option_exercise_price` (`188..201`, implied scale 2)
    - `price_correction_indicator` (`201..202`)
    - `maturity_date` (`202..210`, `YYYYMMDD`, `00000000` / `99991231` $\rightarrow$ `None`)
    - `quotation_factor` (`210..217`, integer `FATCOT`)
    - `exercise_price_points` (`217..230`, implied scale 6)
    - `isin` (`230..242`)
    - `distribution_number` (`242..245`, `DISMES`)
  - Record type `99` (bytes `0..2`): Trailer (`31..42` `TOTREG` total lines — accepts `len(rows)` or `len(rows) + 2` cleanly).

**Natural Key:**
- `UNIQUE NULLS NOT DISTINCT (trade_date, ticker_symbol, market_type, bdi_code, distribution_number)`.

---

## 7. Data Model & Database Contracts

### 7.1 Operational & Provenance Tables

1. **`b3_pending_updates` (Updates Scanner Table)**
   - Fields: `id`, `source_file_type`, `reference_date`, `source_url`, `remote_etag`, `remote_last_modified`, `remote_content_length`, `detected_at`, `status` (`pending`, `ingested`, `ignored`, `failed`), `sync_run_id`.

2. **`b3_source_files`**
   - Fields: `id`, `source_file_type`, `source_name`, `reference_date`, `source_url`, `original_file_name`, `local_path`, `sha256`, `byte_size`, `downloaded_at`, `parsed_at`, `ingested_at`, `parser_version`, `status` (`downloaded`, `parsed`, `ingested`, `skipped_duplicate`, `failed`), `member_count`, `row_count_raw`, `row_count_inserted`, `row_count_changed`, `row_count_unchanged`, `row_count_quarantined`, `error_message`, `created_at`, `updated_at`.

3. **`b3_source_file_members` (Two-Phase Multi-Member Tracking)**
   - Fields: `id`, `source_file_id`, `member_name`, `member_index`, `sha256`, `byte_size`, `status`, `row_count_raw`, `row_count_inserted`, `row_count_changed`, `row_count_unchanged`, `row_count_quarantined`, `processed_at`.

4. **`b3_sync_runs`**
   - Fields: `id`, `job_type` (`scheduled_daily`, `manual_sync`, `cotahist_backfill`, `rebuild_identity`, `quarantine_replay`), `requested_by`, `started_at`, `finished_at`, `status`, `source_file_type`, `reference_date_from`, `reference_date_to`, `source_file_ids` (JSONB), `stats_json` (JSONB), `created_at`, `updated_at`.

5. **`b3_quarantine_records` (Quarantine Table)**
   - Fields: `id`, `sync_run_id`, `source_file_id`, `source_member_name`, `source_file_type`, `reference_date`, `source_line_number`, `reason_code`, `error_message`, `raw_payload` (JSONB), `reprocessed_at`, `resolution_status` (`quarantined`, `reprocessed`, `discarded`), `created_at`.

6. **`b3_field_changes` (Active Field-Level Audit Table)**
   - Actively populated inside `changed_field_only_upsert` whenever an existing record's business field changes.
   - Fields: `id`, `sync_run_id`, `source_file_id`, `table_name`, `record_pk`, `field_name`, `old_value_text`, `new_value_text`, `old_value_hash`, `new_value_hash`, `changed_at`, `created_at`, `updated_at`.

7. **`b3_trading_calendar`**
   - Fields: `calendar_date` (PK), `is_trading_day`, `holiday_name`, `source_note`, `updated_at`.

### 7.2 Domain Normalized Tables
- `b3_instruments`
- `b3_instrument_identifiers`
- `b3_tradable_securities` & `b3_tradable_securities_staging`
- `b3_price_report_quotes`
- `b3_equities_simplified_price_quotes`
- `b3_indices`
- `b3_index_report_values`
- `b3_index_portfolio_compositions`
- `b3_cotahist_quotes`

### 7.3 Internal Change Timestamp & Provenance Contract

Every normalized domain table includes:
- `source_file_id varchar(36) not null`
- `source_member_name varchar(255) null`
- `source_line_number integer null`
- `source_row_hash varchar(64) not null`
- `created_at timestamptz not null`
- `updated_at timestamptz not null`
- `last_seen_at timestamptz not null`
- `data_changed_at timestamptz not null`
- `field_changed_at jsonb not null default '{}'`

**Rules:**
1. **On Insert**: `created_at`, `last_seen_at`, and `data_changed_at` are set to the ingestion UTC timestamp, and `field_changed_at` maps every business field key to that ISO timestamp.
2. **On Identical Row (`source_row_hash` matches)**: No business field update occurs. `data_changed_at`, `field_changed_at`, and `source_row_hash` **remain strictly untouched**.
3. **On Changed Row (`source_row_hash` differs)**:
   - Compare canonicalized business fields.
   - Update only the changed columns.
   - For each changed field `x`, update `field_changed_at['x'] = now.to_rfc3339()` and insert a corresponding audit record into `b3_field_changes`.
   - Update `data_changed_at = now`, `last_seen_at = now`, `source_row_hash = new_hash`, and `source_file_id = current_source_file_id`.

### 7.4 Value Canonicalization in Rust
- **Strings**: Trim ASCII/Unicode outer whitespace, preserve internal single spaces, convert empty strings `""` to `None`.
- **Tickers & ISINs**: Uppercase ASCII (`PETR4`, `BRPETRACNPR6`).
- **Dates**: `chrono::NaiveDate` serialized as ISO `YYYY-MM-DD`. Sentinel dates (`00000000`, `99991231`) canonicalize to `None`.
- **Timestamps**: `chrono::DateTime<chrono::Utc>`.
- **Decimals**: `rust_decimal::Decimal` (`normalize()` stripped trailing zeros for canonical hashing; formatted as canonical decimal string in JSON API responses).
- **Row Hashing**: `sha2::Sha256` over a deterministically ordered `BTreeMap<String, CanonicalValue>` JSON representation.

---

## 8. Ingestion, Two-Phase Pipeline, and Relationship Rules

### 8.1 Required Ingestion Ordering
1. `BVBG.028.02 Instruments File` (or `InstrumentsConsolidated`)
2. `Tradable Security List`
3. `BVBG.087.01 IndexReport` and `B3 Index Portfolio Composition`
4. `BVBG.086.01 PriceReport`
5. `BVBG.186.01 Simplified Price Report - Equities`
6. `COTAHIST` (`Daily`, `Monthly`, `Yearly` backfills)
7. `Identity Resolution Rebuild` (automatic or manual post-backfill pass)

### 8.2 Two-Phase Ingestion Architecture
- **Phase 1 (Acquisition, Integrity & Member Inventory)**:
  - Validate host against `B3_ALLOWED_HOSTS` (`www.b3.com.br`, `bvmf.bmfbovespa.com.br`, `arquivos.b3.com.br`, `sistemaswebb3-listados.b3.com.br`) or local `B3_RAW_DATA_DIR` file path.
  - Download archive, compute `sha256`, persist raw file to `B3_RAW_DATA_DIR` when configured.
  - Check `b3_source_files` for duplicate `(source_file_type, reference_date, sha256)`. If already `ingested` and `force_reparse=false`, create a `skipped_duplicate` record and exit immediately.
  - Recursively inspect ZIP archives, extracting and registering all valid inner XML/CSV/TXT members in `b3_source_file_members` in deterministic order.
- **Phase 2 (Streaming Parse, Quarantine & Upsert)**:
  - Parse records from each member using streaming `quick-xml` or fixed-width/CSV iterators.
  - Isolate malformed lines/nodes into `b3_quarantine_records` with `source_line_number` and `reason_code` without aborting valid rows in the file.
  - Resolve `instrument_id` deterministically and execute changed-field-only upserts, emitting `b3_field_changes` rows whenever a business attribute changes.

### 8.3 Relationship Anchors & Retroactive Resolution
- **Primary Anchor**: B3 `security_id` from Trade Structure (`BVBG.028.02`).
- **Secondary Anchors**:
  1. `isin + ticker_symbol + market_code/market_type + trade_date` within `[valid_from, valid_to]`.
  2. `ticker_symbol + market_code/market_type + trade_date` within `[valid_from, valid_to]`.
  3. `isin + trade_date` when unique.
- **Forbidden Anchors**: `CNPJ_CIA`, `CD_CVM`, issuer CNPJ, CVM code, Yahoo symbol, broker internal code.
- **Retroactive Rebuild (`POST /api/v1/b3/sync-runs/rebuild-identity`)**:
  - Re-evaluates rows where `resolution_status IN ('unresolved', 'ambiguous')` across all downstream tables (promoting resolved rows from `b3_tradable_securities_staging` into `b3_tradable_securities` and linking `instrument_id` on quotes/compositions).

---

## 9. Worker, Scheduler, and Updates Service Requirements

- **PostgreSQL-Backed Coordination (Zero Redis Dependency)**:
  - The background worker (`tucano-b3 worker` or embedded background task loop) coordinates via PostgreSQL advisory locks (`pg_try_advisory_lock(hashtext(lock_key))`) and `b3_sync_runs` status rows.
  - Concurrent attempts to sync the same `source_file_type:reference_date` return HTTP `409 Conflict` immediately.
- **B3 Trading Calendar Gate**:
  - Scheduled daily tasks check `b3_trading_calendar` (and weekend checks); on non-trading days, scheduled runs skip external downloads cleanly unless `force_download=true`.

---

## 10. API & Analytical MCP Requirements

### 10.1 REST API Conventions
- Base path: `/api/v1`
- Authentication: Bearer token (`TUCANO_B3_TOKEN`) on protected routes; `/metrics` and `/docs` / `/openapi.json` accessible for observability and developer discovery.
- Deterministic Pagination & Sorting: All list endpoints support `limit` (default 100, max 1000), `offset`, and deterministic ordering with a unique tie-breaker.
- Decimal Serialization: Canonical strings without thousand separators (e.g., `"31.70"`), matching `rust_decimal` string formatting.
- Every response item includes `provenance` (`source_file_type`, `source_file_id`, `source_file_name`, `source_member_name`, `reference_date`, `sha256`, `ingested_at`).
- When `include_internal=true`, response items also include `internal_metadata` (`data_changed_at`, `field_changed_at`, `resolution_status`, `last_seen_at`, `source_line_number`).

### 10.2 Source-Specific REST Endpoints

1. **Infrastructure & Health**
   - `GET /api/v1/health`
   - `GET /api/v1/b3/worker/health`
   - `GET /metrics` (Prometheus metrics)
2. **Instruments File (`BVBG.028.02`)** — Tag: `B3 Instruments File (BVBG.028.02)`
   - `GET /api/v1/b3/instruments`
   - `GET /api/v1/b3/instruments/{instrument_id}`
   - `GET /api/v1/b3/instruments/by-security-id/{security_id}`
   - `GET /api/v1/b3/instruments/by-ticker/{ticker_symbol}`
   - `GET /api/v1/b3/instruments/by-isin/{isin}`
   - `GET /api/v1/b3/instruments/{instrument_id}/identifiers`
3. **Tradable Security List** — Tag: `B3 Tradable Security List`
   - `GET /api/v1/b3/tradable-securities`
   - `GET /api/v1/b3/tradable-securities/by-date/{reference_date}`
   - `GET /api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}`
4. **PriceReport (`BVBG.086.01`)** — Tag: `B3 PriceReport (BVBG.086.01)`
   - `GET /api/v1/b3/price-report/quotes`
   - `GET /api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}`
   - `GET /api/v1/b3/price-report/quotes/by-date/{trade_date}`
   - `GET /api/v1/b3/price-report/quotes/{quote_id}`
5. **Simplified Equities Price Report (`BVBG.186.01`)** — Tag: `B3 Simplified Price Report - Equities (BVBG.186.01)`
   - `GET /api/v1/b3/equities-simplified-price-report/quotes`
   - `GET /api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}`
   - `GET /api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}`
   - `GET /api/v1/b3/equities-simplified-price-report/quotes/{quote_id}`
   - `GET /api/v1/b3/price-report-comparison/{trade_date}`
6. **IndexReport (`BVBG.087.01`)** — Tag: `B3 IndexReport (BVBG.087.01)`
   - `GET /api/v1/b3/index-report/indices`
   - `GET /api/v1/b3/index-report/values`
   - `GET /api/v1/b3/index-report/values/by-index/{index_code}`
   - `GET /api/v1/b3/index-report/values/by-date/{trade_date}`
7. **Index Portfolio Composition** — Tag: `B3 Index Portfolio Composition`
   - `GET /api/v1/b3/index-composition/portfolios`
   - `GET /api/v1/b3/index-composition/portfolios/{index_code}`
   - `GET /api/v1/b3/index-composition/portfolios/{index_code}/by-date/{composition_date}`
   - `GET /api/v1/b3/index-composition/components/by-ticker/{ticker_symbol}`
8. **COTAHIST (`Yearly`, `Monthly`, `Daily`)** — Tag: `B3 COTAHIST Historical Quotes`
   - `GET /api/v1/b3/cotahist/quotes`
   - `GET /api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}`
   - `GET /api/v1/b3/cotahist/quotes/by-isin/{isin}`
   - `GET /api/v1/b3/cotahist/quotes/by-date/{trade_date}`
   - `GET /api/v1/b3/cotahist/quotes/{quote_id}`
9. **Source Files, Sync Jobs, Quarantine, Audit & Coverage** — Tags: `B3 Source Files`, `B3 Sync Jobs`, `B3 Data Quality & Governance`
   - `GET /api/v1/b3/source-files`
   - `GET /api/v1/b3/source-files/{source_file_id}`
   - `GET /api/v1/b3/source-files/{source_file_id}/members`
   - `GET /api/v1/b3/sync-runs`
   - `GET /api/v1/b3/sync-runs/{sync_run_id}`
   - `POST /api/v1/b3/sync-runs`
   - `POST /api/v1/b3/sync-runs/rebuild-identity`
   - `GET /api/v1/b3/quarantine`
   - `POST /api/v1/b3/quarantine/replay`
   - `GET /api/v1/b3/field-changes`
   - `GET /api/v1/b3/pending-updates`
   - `GET /api/v1/b3/diagnostics/coverage`

### 10.3 Read-Only Analytical MCP Server (`tucano-b3 mcp`)
Expose a read-only Model Context Protocol (`MCP`) JSON-RPC server via `stdio` (`tucano-b3 mcp`) and optional HTTP (`POST /mcp`) with tools that reuse the Rust service layer and never mutate data:
- `healthcheck`
- `search_instruments`
- `get_instrument_coverage`
- `get_ohlcv_series`
- `get_index_portfolio`
- `get_dataset_diagnostics`

---

## 11. Unit & Integration Testing Requirements (`cargo test`)

All tests must be written in Rust and executable via `cargo test` locally and inside Docker (`docker compose run --rm api cargo test`):

1. **Parser Unit Tests (`tests/parser_tests.rs`)**:
   - `BVBG.028.02` streaming `quick-xml` parser extracts security ID, market identifier, asset, market, segment, ticker, ISIN, and maturity from both official B3 ISO 20022 XML (`<Instrm><FinInstrmId><OthrId><Id>...`) and simplified XML fixtures.
   - `BVBG.086.01` streaming `quick-xml` parser extracts all price, volume, transaction, and settlement fields from official ISO 20022 `<PricRpt>` blocks (including `@Ccy` attributes) and simplified fixtures.
   - `BVBG.186.01` parser extracts simplified equities fields and tags `BVBG_186_EQUITIES_SIMPLIFIED_PRICE_REPORT`.
   - `BVBG.087.01` parser extracts `IndxInf`, `IOPVInf`, and `BDRInf` records.
   - `COTAHIST` 245-byte parser validates header (`00`), detail (`01`), trailer (`99` with both `len(rows)` and `len(rows) + 2` conventions), implied decimals, `distribution_number` (`DISMES`), `quotation_factor` (`FATCOT`), sentinel dates (`00000000`), and isolates malformed lines into quarantine records without aborting valid lines.
   - Multi-member and nested ZIP extraction processes all inner XML/TXT parts in deterministic sequence.

2. **Canonicalization & Hashing Unit Tests (`tests/canonical_and_hash_tests.rs`)**:
   - Proves decimal normalization (`31.70` == `31.7`), string trimming, ticker/ISIN uppercasing, and deterministic `Sha256` hashing across arbitrary field insertion orders.

3. **Idempotency, Temporal Window & Audit Tests (`tests/upsert_and_audit_tests.rs`)**:
   - Identical source row re-ingestion performs zero normalized updates, emits zero `b3_field_changes` rows, and leaves `data_changed_at` and `field_changed_at` untouched.
   - Single-field change updates only that field, updates `field_changed_at[field]`, updates `data_changed_at`, and inserts a `b3_field_changes` record with old/new text and hashes.
   - Nullable composite natural keys (`NULLS NOT DISTINCT`) prevent duplicate inserts when optional key columns are `NULL`.
   - Daily `BVBG.028.02` ingestion reuses open `b3_instrument_identifiers` windows (`valid_to IS NULL`) and closes `valid_to` when an instrument's ticker changes.

4. **Relationship Resolution & Rebuild Tests (`tests/resolver_and_rebuild_tests.rs`)**:
   - Resolves by `security_id` (priority 1), `isin + ticker + market + date` (priority 2), and `ticker + market + date` (priority 3).
   - Marks conflicting matches as `ambiguous` and missing matches as `unresolved`.
   - Proves `CNPJ_CIA` and `CD_CVM` keys are stripped and ignored.
   - Verifies `rebuild_identity` retroactively resolves previously `unresolved` `COTAHIST` and `Tradable Security List` rows once `BVBG.028.02` is ingested.

5. **API, Auth, Lock & OpenAPI Tests (`tests/api_contract_tests.rs`)**:
   - Verifies Bearer token authentication (`401` when missing/invalid, `200` with valid token).
   - Verifies source-specific endpoints, pagination, `include_internal=true/false`, and `provenance` payload on every item.
   - Verifies OpenAPI JSON (`/openapi.json`) includes source-specific tags, schemas (`B3Instrument`, `B3PriceReportQuote`, `B3CotahistQuote`, etc.), and the raw unadjusted price warning on `COTAHIST`.
   - Verifies concurrent sync lock contention (`pg_try_advisory_lock`) and duplicate file SHA-256 skipping (`skipped_duplicate`).

---

## 12. Docker & Docker Compose Requirements

### 12.1 Multi-Stage Rust `Dockerfile`
- **`builder` stage (`rust:1.82-bookworm` or `rust:1-bookworm`)**:
  - Caches dependency compilation (`cargo build --release`) and compiles the `tucano-b3` binary + test suite.
- **`development` / `test` target**:
  - Allows running `docker compose run --rm api cargo test` directly inside containers.
- **`runtime` stage (`debian:bookworm-slim`)**:
  - Installs only `ca-certificates`, `libssl3`, and `curl` (for container healthcheck), copying the compiled `/usr/local/bin/tucano-b3` binary and `/app/migrations`.

### 12.2 `docker-compose.yml` Architecture
Required services:
- **`postgres` (`postgres:16-alpine`)**:
  - Relational database with healthcheck (`pg_isready`) and persistent volume `postgres_data`.
- **`api` (`tucano-b3 serve`)**:
  - Runs embedded SQLx migrations on startup, starts the Axum HTTP server (`0.0.0.0:8000`, mapped to host `8001:8000`), and exposes `/api/v1/...`, `/docs`, `/redoc`, `/openapi.json`, `/metrics`, and `/mcp`.
- **`worker` (`tucano-b3 worker`)**:
  - Runs the background sync processor and timezone-aware daily cron scheduler using the exact same Rust image and shared volumes (`b3_raw_files`, `b3_processed_files`).

Required environment variables:
- `DATABASE_URL` (`postgresql://postgres:postgres@postgres:5432/tucano_b3`)
- `TUCANO_B3_TOKEN`
- `B3_ALLOWED_HOSTS` (default: `www.b3.com.br,bvmf.bmfbovespa.com.br,arquivos.b3.com.br,sistemaswebb3-listados.b3.com.br`)
- `B3_RAW_DATA_DIR` (`/data/raw`)
- `B3_PROCESSED_DATA_DIR` (`/data/processed`)
- `B3_SYNC_ENABLED` (`true`)
- `B3_MANUAL_SYNC_ENABLED` (`true`)
- `B3_DEFAULT_TIMEZONE` (`America/Sao_Paulo`)
- `B3_PARSER_VERSION` (`v1`)
- `RUST_LOG` (`info,tucano_b3=debug`)

Minimum verification commands:
- `docker compose up --build -d`
- `docker compose run --rm api cargo test`
- `curl -H "Authorization: Bearer dev-token" http://localhost:8001/api/v1/health`

---

## 13. Implementation Roadmap (3 Phases)

### Phase 1: Rust Workspace, PostgreSQL Schema (`NULLS NOT DISTINCT`), & Streaming Parsers
- Initialize `Cargo.toml` and PostgreSQL 16 schema migration (`migrations/0001_initial_b3_schema.sql`) with all domain and governance tables (`b3_source_files`, `b3_source_file_members`, `b3_sync_runs`, `b3_instruments`, `b3_instrument_identifiers`, `b3_tradable_securities`, `b3_tradable_securities_staging`, `b3_price_report_quotes`, `b3_equities_simplified_price_quotes`, `b3_indices`, `b3_index_report_values`, `b3_index_portfolio_compositions`, `b3_cotahist_quotes`, `b3_field_changes`, `b3_quarantine_records`, `b3_pending_updates`, `b3_trading_calendar`).
- Implement deterministic canonicalization, `Sha256` row hashing, and streaming parsers (`bvbg_028`, `bvbg_086`, `bvbg_186`, `bvbg_087`, `cotahist`, `tradable_list`, `index_composition`) plus multi-member ZIP extraction.
- Implement changed-field-only upserts with active `b3_field_changes` logging, temporal identifier window closing (`valid_from` / `valid_to`), and `RelationshipResolver` (including `rebuild_identity`).

### Phase 2: Axum REST API, Utoipa OpenAPI 3.1, Worker & Read-Only MCP
- Build all source-specific Axum routers, Bearer auth middleware, Prometheus `/metrics`, and Utoipa OpenAPI 3.1 documentation (`/docs`, `/redoc`, `/openapi.json`).
- Build the two-phase `B3SyncService`, `B3DownloadService` (with multi-host and 2-step token support), PostgreSQL advisory locks, quarantine replay, and `tucano-b3 worker` / `tucano-b3 mcp` entrypoints.

### Phase 3: Comprehensive Rust Test Suite & Docker Compose Verification
- Write unit and database integration tests (`tests/*.rs`) covering all parsers, canonicalization, changed-field upserts, temporal windows, relationship resolution, identity rebuild, locks, and API/OpenAPI contracts.
- Configure multi-stage `Dockerfile` and `docker-compose.yml` (`postgres`, `api`, `worker`), verifying both `cargo test` and end-to-end container execution.
