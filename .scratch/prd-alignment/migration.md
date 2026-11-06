# Tucano B3 - Migration and Remediation Design (Final R6)

Prepared according to [PRD section 19.3](docs/b3_prd.md#193-sequência-de-migração) and [Review Plan](docs/19-plan.md).

Date: 2026-11-06, America/Sao_Paulo  
Scope: Architecture, Data Safeguards, Implementation Sequence, and Release Gates for `tucano-b3`.

---

## 1. Principles of Additive Migration

1. **Immutable Baseline Migration**:
   - `migrations/0001_initial_b3_schema.sql` is frozen and must never be altered retroactively.
   - All future structural additions (e.g., `b3_trading_calendar`, worker task leases, indexes) must be introduced in sequential, additive migrations (`0002_coordination_and_calendar.sql`).
2. **Preservation of Market Facts**:
   - Historical quotes in `b3_cotahist_quotes`, `b3_price_report_quotes`, and `b3_equities_simplified_price_quotes` must never be altered, overwritten, or deduplicated by ticker alone.
   - `NULLS NOT DISTINCT` uniqueness invariants on natural keys guarantee facts remain distinct and uncorrupted.
3. **Audit History Continuity**:
   - Existing records in `b3_field_changes`, `b3_sync_runs`, and `b3_quarantine_records` must remain intact as immutable historical evidence.
4. **Zero-Downtime Consumer Parity**:
   - All 42 public REST endpoints retain their exact URL path (`/api/v1/b3/...`), query parameter names, and response envelope (`data`, `meta`, `provenance`, `unadjusted_warning`).
   - Internal refactorings (such as shared query service extraction) must produce byte-level compatible JSON responses.
5. **No Bypassing of Transports**:
   - MCP and REST must execute identical business and analytical queries via shared service layers.
   - MCP remains strictly read-only and never performs DDL migrations or ingestion mutations.

---

## 2. Implementation & Migration Packages (PRD §19.3 Sequence)

### Package 1: Baseline Preservation & Characterization Tests (PRD Step 1)
- **Status**: Completed in Phase R1.
- **Evidence**:
  - Live OpenAPI 3.1 specification saved in `.scratch/prd-alignment/evidence/openapi_reference.json`.
  - Endpoint and schema inventory recorded in `.scratch/prd-alignment/evidence/contracts_inventory.md`.
  - Representative response scenarios captured in `.scratch/prd-alignment/evidence/response_scenarios.md`.
  - Regression test suite expanded to 35 deterministic tests without skips.

### Package 2: Additive Coordination & Calendar Schema (PRD Steps 2 & 3)
- **Target Migration**: `migrations/0002_coordination_and_calendar.sql`
- **Scope**:
  - Create `b3_trading_calendar`:
    - Columns: `calendar_date DATE PRIMARY KEY`, `is_trading_day BOOLEAN NOT NULL`, `session_type VARCHAR(32) NOT NULL`, `open_time TIME`, `close_time TIME`, `notes TEXT`.
    - Populate with official B3 holiday and special session data from 1986 to present.
  - Create `b3_worker_leases`:
    - Columns: `lease_id UUID PRIMARY KEY`, `resource_name VARCHAR(128) NOT NULL UNIQUE`, `owner_worker_id VARCHAR(128) NOT NULL`, `acquired_at TIMESTAMPTZ NOT NULL`, `heartbeat_at TIMESTAMPTZ NOT NULL`, `lease_duration_seconds INT NOT NULL`.
  - Safe rollback (`down.sql`):
    - Drops `b3_worker_leases` and `b3_trading_calendar` without touching market facts or sync tables.

### Package 3: Streaming Acquisition & Resilient Extraction (PRD Step 4)
- **Scope**:
  - Update `B3DownloadService`:
    - Stream HTTP response bodies directly to disk using `tokio::io::AsyncWriteExt` with bounded buffer chunks (e.g. 64 KB), calculating SHA-256 continuously in flight.
    - Guarantee memory usage remains $< 64\text{ MB}$ even when downloading 500 MB annual COTAHIST archives.
  - Update `extract_archive_members`:
    - Stream ZIP entries to temporary files.
    - Return explicit `AppError::Layout("Maximum ZIP recursion depth (4) exceeded")` instead of silent `Ok(())`.
    - Enforce maximum uncompressed member size limit (e.g., 500 MB).

### Package 4: Transactional Persistence & Session-Lock Fencing (PRD Steps 4 & 5)
- **Scope**:
  - Update `B3SyncService`:
    - Acquire session advisory lock via `try_acquire_session_advisory_lock` on a dedicated checkout connection from `PgPool`.
    - Route all single-date ingestion, parsing, domain fact insertion, `b3_field_changes` auditing, and `b3_sync_runs` status transitions through a single atomic transaction or the locked connection.
    - Release lock deterministically in RAII drop guard or cleanup block upon completion or failure.
    - Quarantine invalid individual records without failing the entire valid batch.

### Package 5: Shared Query Services Layer (PRD Step 6)
- **Scope**:
  - Extract query SQL duplicated between `src/api/routers/` and `src/mcp/tools.rs` into:
    - `src/services/query/instruments.rs`: search, detail, temporal coverage, corporate actions.
    - `src/services/query/quotes.rs`: daily OHLCV series, price reports, simplified quotes, raw price warning envelope.
    - `src/services/query/indices.rs`: theoretical portfolio compositions, index weights.
    - `src/services/query/governance.rs`: companies, economic groups, sectors.
  - Wire Axum handlers and MCP tools to call these shared services.
  - Guarantee 100% parameter, ordering, pagination, and envelope parity between transports.

### Package 6: Worker Scheduler & Discovery Decoupling (PRD Step 6)
- **Scope**:
  - Replace `Local::now()` and Monday–Friday checks in `src/worker/mod.rs` with:
    - `chrono_tz::America::Sao_Paulo` timezone enforcement.
    - Query against `b3_trading_calendar` before triggering daily syncs.
    - Decouple discovery scanner (`b3_pending_updates`) from heavy ingestion.
    - Implement heartbeat update in `b3_worker_leases` during sync execution.

### Package 7: Shadow Comparison, Cutover, and Retirement (PRD Step 7)
- **Scope**:
  - Run shadow pipeline alongside existing pipeline on test datasets (`COTAHIST_A2026.TXT`, `IN260301.zip`).
  - Verify zero discrepancy in `stable_row_hash`, inserted counts, and audit logs.
  - Switch default execution path to streaming/shared query pipeline.
  - Deprecate old unbuffered handlers once full parity is verified.

---

## 3. Shadow Comparison and Cutover Protocol

```mermaid
sequenceDiagram
    autonumber
    participant Raw as Raw B3 Files
    participant Shadow as Enhanced Streaming Pipeline
    participant DB as PostgreSQL 16 (Shadow Schema)
    participant Verifier as Shadow Comparison Harness
    participant Prod as Production Pipeline

    Raw->>Shadow: Ingest via Streaming I/O (Disk-buffered)
    Shadow->>DB: Atomically persist facts + audits in transaction
    Raw->>Prod: Legacy Ingestion
    Prod->>DB: Persist legacy baseline
    Verifier->>DB: Compare natural keys, stable_row_hash, field changes
    Note over Verifier,DB: Verification Gate: 100% hash parity & memory < 256MB
    Verifier-->>Prod: Authorize Cutover (Promote Streaming Pipeline)
```

### Verification Criteria Before Promotion
1. **Fact Equality**: 100% of rows inserted by the enhanced pipeline must have identical `stable_row_hash` to the legacy pipeline for the same input files.
2. **Audit Equality**: `b3_field_changes` must record identical field-level diffs.
3. **Memory Ceiling**: RSS memory during ingestion of full annual COTAHIST file (e.g. `COTAHIST_A2026.TXT`) must not exceed 256 MB.
4. **Lock Invariant**: Concurrent executions attempting the same date/source must return clean conflict status (HTTP 409 or skip) without deadlocking or leaking locks.

---

## 4. Review Plan Milestones vs. PRD Release Gates

### Review Plan Execution Status (docs/19-plan.md)
- [x] **R0 Baseline**: Hardened integration tests and isolated PostgreSQL 16 development environment.
- [x] **R1 Inventory**: Live OpenAPI 3.1 contract reference extracted and response scenarios documented.
- [x] **R2 Parsers**: ISO 20022 schemas and depth limit tests added and verified.
- [x] **R3 Persistence**: `UNIQUE NULLS NOT DISTINCT` invariant verified on natural keys.
- [x] **R4 Coordination**: Transactional fencing and session lock helpers implemented and verified.
- [x] **R5 Transports**: MCP read-only startup verified without migration rights.
- [x] **R6 Consolidation**: Subsystem dispositions recorded; migration roadmap and issue tracking finalized.

### PRD Release Gates Status (PRD §19 / §20)
- [ ] **Gate G0 (Source Viability)**: `needs-info` — External collection disabled awaiting legal eligibility confirmation ([Issue 01](docs/issues/01-source-viability.md)).
- [ ] **Gate G1 (Domain & Contracts)**: `pending-spec` — Contract baseline established; formal qualifier and temporal window specification pending ([Issue 02](docs/issues/02-domain-and-contracts.md)).
- [ ] **Gate G2 (Acquisition & Artifacts)**: `blocked` — Blocked by P1 in-memory download buffering and silent ZIP depth truncation ([Issue 03](docs/issues/03-artifact-acquisition.md)).
- [ ] **Gate G3 (Historical Core)**: `blocked` — Blocked by P1 persistence gap (lack of atomic transaction boundary across facts, audits, and status) and permissive COTAHIST trailer check ([Issue 04](docs/issues/04-historical-core.md)).
- [ ] **Gate G4 (Query Product)**: `blocked` — Blocked by P2 architecture gap (lack of shared query services between REST and MCP) ([Issue 05](docs/issues/05-query-product.md)).
- [ ] **Gate G5 (Operations)**: `blocked` — Blocked by P1 coordination gap (lack of `b3_trading_calendar`, worker timezone drift, and lack of retained connection locks in sync service) ([Issue 06](docs/issues/06-operations.md)).
- [ ] **Gate G6 (Price Reports)**: `blocked` — Blocked on platform prerequisites G1–G5 ([Issue 07](docs/issues/07-price-reports.md)).
- [ ] **Gate G7 (Indices & Portfolios)**: `blocked` — Blocked on platform prerequisites G1–G5 ([Issue 08](docs/issues/08-indices-and-portfolios.md)).
