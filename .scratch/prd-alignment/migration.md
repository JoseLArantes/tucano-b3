# Tucano B3 - Migration and Remediation Design (Final R6)

Prepared according to [PRD section 19.3](docs/b3_prd.md#193-sequência-de-migração) and [Review Plan](docs/19-plan.md).

Date: 2026-11-06, America/Sao_Paulo  
Scope: Architecture, Data Safeguards, Implementation Sequence, and Release Gates for `tucano-b3`.

---

## 1. Principles of Additive Migration

1. **Immutable Baseline Migration**:
   - `migrations/0001_initial_b3_schema.sql` is frozen and must never be altered retroactively.
   - All future structural additions or column enhancements must be introduced in sequential, additive migrations (`0002_coordination_and_calendar.sql`).
2. **Preservation of Market Facts**:
   - Historical quotes in `b3_cotahist_quotes`, `b3_price_report_quotes`, and `b3_equities_simplified_price_quotes` must never be altered, overwritten, or deduplicated by ticker alone.
   - `NULLS NOT DISTINCT` uniqueness invariants on natural keys guarantee facts remain distinct and uncorrupted.
3. **Audit History Continuity**:
   - Existing records in `b3_field_changes`, `b3_sync_runs`, and `b3_quarantine_records` must remain intact as immutable historical evidence.
4. **Zero-Downtime Consumer Parity**:
   - All 44 public/protected REST endpoints retain their exact URL path (`/api/v1/b3/...`), query parameter names, and standard response envelope:
     ```json
     {
       "items": [ ... ],
       "total": 1520,
       "limit": 100,
       "offset": 0,
       "disclaimer": "..."
     }
     ```
   - Internal refactorings (such as shared query service extraction) must produce byte-level compatible JSON responses matching `PaginatedResponse<T>`.
5. **Strict Transport Boundary**:
   - MCP and REST must execute identical analytical queries via shared service layers (`src/services/query/`).
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
- **Context & Safeguards**:
  - `b3_trading_calendar` was **already created** in `migrations/0001_initial_b3_schema.sql` with columns `(calendar_date DATE PRIMARY KEY, is_trading_day BOOLEAN NOT NULL DEFAULT TRUE, holiday_name VARCHAR(255), source_note VARCHAR(255), updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW())`.
  - **Never drop or recreate `b3_trading_calendar`**: Doing so would corrupt existing baseline schemas and destroy calendar data on rollback.
- **Additive Scope**:
  - Enhance `b3_trading_calendar` with non-destructive, additive columns:
    ```sql
    ALTER TABLE b3_trading_calendar ADD COLUMN IF NOT EXISTS session_type VARCHAR(32) NOT NULL DEFAULT 'REGULAR';
    ALTER TABLE b3_trading_calendar ADD COLUMN IF NOT EXISTS settlement_day BOOLEAN NOT NULL DEFAULT TRUE;
    ```
  - Populate/seed official B3 trading calendar records (1986–present), including national holidays, Carnival, Corpus Christi, and special half-day sessions (Ash Wednesday).
  - Create `b3_worker_leases` for durable task claiming and heartbeat fencing:
    ```sql
    CREATE TABLE IF NOT EXISTS b3_worker_leases (
        lease_id UUID PRIMARY KEY,
        resource_name VARCHAR(128) NOT NULL UNIQUE,
        owner_worker_id VARCHAR(128) NOT NULL,
        acquired_at TIMESTAMPTZ NOT NULL,
        heartbeat_at TIMESTAMPTZ NOT NULL,
        lease_duration_seconds INT NOT NULL
    );
    ```
  - **Active Revision Pointer & Status Normalization Schema**:
    - Add explicit active revision tracking to `b3_source_files` in `0002_coordination_and_calendar.sql`:
      ```sql
      ALTER TABLE b3_source_files
          ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT false,
          ADD COLUMN IF NOT EXISTS revision_number INT NOT NULL DEFAULT 1;

      CREATE UNIQUE INDEX IF NOT EXISTS uq_source_files_active_revision
          ON b3_source_files (source_file_type, reference_date)
          WHERE is_active = true;
      ```
  - **Revision-Scoped Uniqueness & Public ID Preservation Schema**:
    - Add `public_id VARCHAR(36)` to all 6 fact tables, backfilled from existing `id` so public URLs and bookmarks are preserved across revisions:
      ```sql
      ALTER TABLE b3_cotahist_quotes ADD COLUMN IF NOT EXISTS public_id VARCHAR(36);
      UPDATE b3_cotahist_quotes SET public_id = id WHERE public_id IS NULL;
      ALTER TABLE b3_cotahist_quotes ALTER COLUMN public_id SET NOT NULL;
      CREATE INDEX IF NOT EXISTS idx_cotahist_public_id ON b3_cotahist_quotes(public_id);
      ```
      (Repeated for `b3_price_report_quotes`, `b3_equities_simplified_price_quotes`, `b3_index_report_values`, `b3_index_portfolio_compositions`, and `b3_tradable_securities`).
    - Migrate natural key uniqueness constraints from single-scope to **revision-scoped** (`source_file_id` included):
      ```sql
      -- COTAHIST
      ALTER TABLE b3_cotahist_quotes DROP CONSTRAINT IF EXISTS uq_cotahist_nk;
      ALTER TABLE b3_cotahist_quotes ADD CONSTRAINT uq_cotahist_revision_nk
          UNIQUE NULLS NOT DISTINCT (source_file_id, trade_date, ticker_symbol, market_type, bdi_code, distribution_number);

      -- PriceReport (BVBG.086)
      ALTER TABLE b3_price_report_quotes DROP CONSTRAINT IF EXISTS uq_price_report_nk;
      ALTER TABLE b3_price_report_quotes ADD CONSTRAINT uq_price_report_revision_nk
          UNIQUE NULLS NOT DISTINCT (source_file_id, trade_date, security_id, ticker_symbol, market_identifier_code, days_to_settlement);

      -- Equities Simplified (BVBG.186)
      ALTER TABLE b3_equities_simplified_price_quotes DROP CONSTRAINT IF EXISTS uq_equities_simplified_nk;
      ALTER TABLE b3_equities_simplified_price_quotes ADD CONSTRAINT uq_equities_simplified_revision_nk
          UNIQUE NULLS NOT DISTINCT (source_file_id, trade_date, security_id, ticker_symbol, market_identifier_code);

      -- IndexReport Values (BVBG.087)
      ALTER TABLE b3_index_report_values DROP CONSTRAINT IF EXISTS uq_index_report_nk;
      ALTER TABLE b3_index_report_values ADD CONSTRAINT uq_index_report_revision_nk
          UNIQUE NULLS NOT DISTINCT (source_file_id, trade_date, index_code, value_type);

      -- Index Portfolio Composition
      ALTER TABLE b3_index_portfolio_compositions DROP CONSTRAINT IF EXISTS uq_index_composition_nk;
      ALTER TABLE b3_index_portfolio_compositions ADD CONSTRAINT uq_index_composition_revision_nk
          UNIQUE NULLS NOT DISTINCT (source_file_id, composition_date, index_code, ticker_symbol, instrument_id);

      -- Tradable Securities
      ALTER TABLE b3_tradable_securities DROP CONSTRAINT IF EXISTS uq_tradable_nk;
      ALTER TABLE b3_tradable_securities ADD CONSTRAINT uq_tradable_revision_nk
          UNIQUE NULLS NOT DISTINCT (source_file_id, reference_date, security_id, ticker_symbol, market_code);
      ```
  - **Reconstruction & Validation of Complete Revision Membership (Pre-Filter Data Migration)**:
    - **Defect in Naive `created_at DESC`**: Current ingestion uses `ON CONFLICT DO NOTHING`. A reprocessed file can be marked completed while all persisted facts remain attached to the older file. Sorting by `created_at DESC` activates an empty file and deactivates the file actually holding the facts, causing 100% data loss from reader queries.
    - **Reconstructive Membership Procedure**:
      1. Normalize operational statuses:
         ```sql
         UPDATE b3_source_files SET status = 'completed' WHERE status = 'ingested';
         UPDATE b3_sync_runs SET status = 'completed' WHERE status = 'success';
         ```
      2. Identify the true fact-holding file by auditing persisted facts in domain tables:
         ```sql
         WITH fact_counts AS (
             SELECT source_file_id, COUNT(*) AS cnt FROM b3_cotahist_quotes GROUP BY source_file_id
             UNION ALL
             SELECT source_file_id, COUNT(*) AS cnt FROM b3_price_report_quotes GROUP BY source_file_id
             UNION ALL
             SELECT source_file_id, COUNT(*) AS cnt FROM b3_equities_simplified_price_quotes GROUP BY source_file_id
             UNION ALL
             SELECT source_file_id, COUNT(*) AS cnt FROM b3_index_report_values GROUP BY source_file_id
             UNION ALL
             SELECT source_file_id, COUNT(*) AS cnt FROM b3_index_portfolio_compositions GROUP BY source_file_id
             UNION ALL
             SELECT source_file_id, COUNT(*) AS cnt FROM b3_tradable_securities GROUP BY source_file_id
         ),
         file_fact_totals AS (
             SELECT sf.id, sf.source_file_type, sf.reference_date, COALESCE(SUM(fc.cnt), 0) AS total_facts
             FROM b3_source_files sf
             LEFT JOIN fact_counts fc ON fc.source_file_id = sf.id
             WHERE sf.status = 'completed' AND sf.reference_date IS NOT NULL
             GROUP BY sf.id, sf.source_file_type, sf.reference_date
         ),
         fact_bearing_files AS (
             -- Select the file that actually holds the populated facts (> 0)
             SELECT DISTINCT ON (source_file_type, reference_date) id
             FROM file_fact_totals
             WHERE total_facts > 0
             ORDER BY source_file_type, reference_date, total_facts DESC, id
         )
         UPDATE b3_source_files sf
         SET is_active = true
         FROM fact_bearing_files fbf
         WHERE sf.id = fbf.id;
         ```
      3. **Pre-Filter Validation Gate**:
         - Execute an explicit assertion ensuring that every scope with persisted facts has an active source file and zero orphaned facts:
           ```sql
           DO $$
           DECLARE
               orphan_count INT;
           BEGIN
               SELECT COUNT(*) INTO orphan_count
               FROM b3_cotahist_quotes q
               JOIN b3_source_files sf ON sf.id = q.source_file_id
               WHERE sf.is_active = false
                 AND EXISTS (
                     SELECT 1 FROM b3_source_files active_sf
                     WHERE active_sf.source_file_type = sf.source_file_type
                       AND active_sf.reference_date = sf.reference_date
                       AND active_sf.is_active = true
                 );
               IF orphan_count > 0 THEN
                   RAISE EXCEPTION 'Pre-filter validation failed: % orphaned facts found under inactive files.', orphan_count;
               END IF;
           END $$;
           ```
  - **Safe Rollback Strategy (`down.sql`)**:
    - Drops revision-scoped natural keys and restores original natural key constraints.
    - Drops `idx_*_public_id` and `public_id` columns from fact tables.
    - Drops `uq_source_files_active_revision`, `revision_number`, `is_active`, and `b3_worker_leases`.
    - Removes only the added columns from `b3_trading_calendar`, preserving the base table and its rows:
      ```sql
      ALTER TABLE b3_trading_calendar DROP COLUMN IF EXISTS settlement_day;
      ALTER TABLE b3_trading_calendar DROP COLUMN IF EXISTS session_type;
      ```

### Package 3: Streaming Acquisition & Resilient Extraction (PRD Step 4)
- **Scope**:
  - Update `B3DownloadService`:
    - Stream HTTP response bodies directly to disk using `tokio::io::AsyncWriteExt` with bounded buffer chunks (e.g. 64 KB), calculating SHA-256 continuously in flight.
    - Guarantee memory usage remains $< 64\text{ MB}$ even when downloading 500 MB annual COTAHIST archives.
  - Update `extract_archive_members`:
    - Stream ZIP entries to temporary files.
    - Return explicit `AppError::Layout("Maximum ZIP recursion depth (4) exceeded")` instead of silent `Ok(())`.
    - Enforce maximum uncompressed member size limit (e.g., 500 MB).

### Package 4: Transactional Persistence, Versioned Fact Preservation & Active-Revision Publication (PRD Steps 4 & 5)
- **Publication Scope Definition**:
  - The publication boundary is strictly the **Complete Source File Scope** (the source archive and 100% of its mandatory members for a given `source_file_type` and `reference_date`).
  - **Zero Partial-Member Leakage**: A multi-member archive must either publish all required members together or publish nothing. Individual members must NEVER be committed independently to domain tables if a subsequent member fails.
- **Architecture & Invariants**:
  1. **Concurrency Exclusion**:
     - Acquire a session-level advisory lock (`pg_try_advisory_lock`) on a dedicated checkout connection from `PgPool` across the task duration.
     - Guarantees mutual exclusion between workers contending on the same file/date. Provides exclusion only, not transactional rollback.
  2. **Preservation of Versioned Facts & Revision-Scoped Uniqueness**:
     - All fact records (`b3_cotahist_quotes`, `b3_price_report_quotes`, `b3_equities_simplified_price_quotes`, `b3_index_report_values`, `b3_index_portfolio_compositions`, `b3_tradable_securities`) retain their immutable link to `source_file_id`.
     - Natural key uniqueness constraints are scoped to `(source_file_id, natural_key)` via `uq_*_revision_nk`.
     - Ingesting a candidate revision stages new facts linked to `candidate_source_file_id` without triggering unique constraint violations against existing active revisions.
     - **Preservation of Public IDs**:
       - Each fact row has a surrogate version PK `id` and a stable public identifier `public_id`.
       - For candidate revisions, `public_id` is retained from the previous version of that business fact (or generated deterministically via UUIDv5 from the business natural key).
       - REST endpoints (e.g. `/quotes/{quote_id}`) filter by `(q.public_id = $1 OR q.id = $1) AND sf.is_active = true`, ensuring existing client URLs and bookmarks remain 100% valid.
     - Prior revision facts remain intact and queryable under the existing active revision pointer until the candidate revision is fully validated and published.
     - **Rollback Guarantee**: If an active revision is superseded or needs revocation, previous facts are never destroyed: restoring the previous revision is an instantaneous, atomic pointer reactivation (`is_active = true` on prior `source_file_id`, `is_active = false` on superseded) without re-downloading or re-parsing raw archives.
  3. **Explicit Active Revision Pointer per Scope**:
     - Scope is uniquely defined as `(source_file_type, reference_date)`.
     - An explicit partial unique index (`uq_source_files_active_revision`) guarantees at the database engine level that at most ONE file revision is active (`is_active = true`) per scope at any time.
     - Reader queries in `src/services/query/` must join against the active revision pointer, NEVER on raw `status = 'completed'` (which admits superseded revisions):
       ```sql
       JOIN b3_source_files sf ON sf.id = fact.source_file_id AND sf.is_active = true
       ```
     - Readers observe exactly one unambiguous, fully validated revision per scope.
  4. **Data Migration Precondition**:
     - Before introducing the `sf.is_active = true` filter into `src/services/query/`, migration `0002` normalizes legacy `ingested`/`success` states to `completed` and reconstructs/validates complete revision membership based on actual persisted fact counts rather than `created_at`.
  5. **Atomic Publication Pointer Flip**:
     - Inside the complete-file transaction (holding the session advisory lock):
       1. Insert candidate fact rows linked to `candidate_source_file_id`.
       2. Validate trailer counts, member schemas, and record field audits in `b3_field_changes`.
       3. Atomically switch the active revision pointer:
          ```sql
          -- Deactivate currently active revision for this scope
          UPDATE b3_source_files
          SET is_active = false, updated_at = NOW()
          WHERE source_file_type = $1 AND reference_date = $2 AND is_active = true;

          -- Activate candidate revision and mark completed
          UPDATE b3_source_files
          SET is_active = true, status = 'completed', updated_at = NOW()
          WHERE id = $3;
          ```
       4. Update `b3_sync_runs` to `status = 'completed'`.
       5. Issue transaction `COMMIT`.
     - **Failure Rollback**: If any member validation fails or an unrecoverable error occurs, the transaction issues `ROLLBACK`. All candidate facts are discarded by PostgreSQL simultaneously, leaving the prior active revision intact and serving queries without interruption.
  6. **Lock Lifecycle**: The session advisory lock is released in an RAII drop guard or explicit `finally` block before returning the connection to the pool.

### Package 5: Shared Query Services Layer (PRD Step 6)
- **Scope**:
  - Extract query SQL duplicated between `src/api/routers/` and `src/mcp/tools.rs` into:
    - `src/services/query/instruments.rs`: search, detail by security_id/ticker/isin, temporal identifiers history, and coverage diagnostics.
    - `src/services/query/quotes.rs`: COTAHIST raw historical OHLCV series with unadjusted prices disclaimer, BVBG.086 PriceReport quotes, BVBG.186 simplified quotes, and 086 vs 186 price comparison.
    - `src/services/query/indices.rs`: BVBG.087 registered indices list, index close/settlement values by date/code, theoretical portfolio compositions by index and date, and reverse ticker membership.
    - `src/services/query/source_files.rs`: source files catalog, archive member details, ingestion sync runs status/counters, and quarantine records inspection.
  - Wire Axum handlers and MCP tools to call these shared services.
  - Guarantee 100% parameter, ordering, pagination (`PaginatedResponse<T>`), and envelope parity between transports.
  - **Scope Boundary**: Corporate actions, external CVM enrichment, economic groups, and sector classifications are strictly excluded from this query layer, in full compliance with `AGENTS.md` and `PRODUCT.md`.

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
    Note over Shadow,DB: Atomically persist facts + audits in explicit transaction
    Shadow->>DB: BEGIN -> INSERT facts/audits -> COMMIT
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
- [ ] **R5 Transports**: Static verification of migration bypass in `src/main.rs:52` on `Commands::Mcp`; runtime process test with restricted read-only PostgreSQL role remains pending.
- [x] **R6 Consolidation**: Subsystem dispositions recorded; migration roadmap and issue tracking finalized.

### PRD Release Gates Status (PRD §19 / §20)
- [ ] **Gate G0 (Source Viability)**: `needs-info` — External collection disabled awaiting legal eligibility confirmation ([Issue 01](docs/issues/01-source-viability.md)).
- [ ] **Gate G1 (Domain & Contracts)**: `pending-spec` — Contract baseline established; formal qualifier and temporal window specification pending ([Issue 02](docs/issues/02-domain-and-contracts.md)).
- [ ] **Gate G2 (Acquisition & Artifacts)**: `blocked` — Blocked by P1 in-memory download buffering and silent ZIP depth truncation ([Issue 03](docs/issues/03-artifact-acquisition.md)).
- [ ] **Gate G3 (Historical Core)**: `blocked` — Blocked by P1 persistence gap (lack of atomic transaction boundary across facts, audits, and status) and permissive COTAHIST trailer check ([Issue 04](docs/issues/04-historical-core.md)).
- [ ] **Gate G4 (Query Product)**: `blocked` — Blocked by P2 architecture gap (lack of shared query services between REST and MCP) ([Issue 05](docs/issues/05-query-product.md)).
- [ ] **Gate G5 (Operations)**: `blocked` — Blocked by P1 coordination gap (lack of populated trading calendar, worker timezone drift, and lack of durable worker leases) ([Issue 06](docs/issues/06-operations.md)).
- [ ] **Gate G6 (Price Reports)**: `blocked` — Blocked on platform prerequisites G1–G5 ([Issue 07](docs/issues/07-price-reports.md)).
- [ ] **Gate G7 (Indices & Portfolios)**: `blocked` — Blocked on platform prerequisites G1–G5 ([Issue 08](docs/issues/08-indices-and-portfolios.md)).
