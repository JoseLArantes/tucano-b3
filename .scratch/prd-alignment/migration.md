# Tucano B3 - Migration and Remediation Design (R6)

Prepared according to [PRD section 19.3](docs/b3_prd.md#193-sequência-de-migração) and [Review Plan](docs/19-plan.md).

---

## 1. Principles of Additive Migration

1. **Immutable Baseline Schema**: `migrations/0001_initial_b3_schema.sql` remains completely untouched. All new tables, indexes, or altered constraints are added via sequential migrations (`0002_...sql`).
2. **Preservation of Market Facts**: Historical quotes in `b3_cotahist_quotes`, `b3_price_report_quotes`, and `b3_equities_simplified_price_quotes` must never be altered or deduplicated by ticker alone.
3. **Audit Continuity**: Existing records in `b3_field_changes` and `b3_sync_runs` remain intact as historical evidence.
4. **Zero Downtime / Consumer Parity**: All 42 public REST endpoints retain their exact URL structure, parameter names, and response envelope. Internal refactorings (such as shared query extraction) must produce byte-level compatible JSON responses.

---

## 2. Planned Migration Steps

### Step 1: Additive Schema Enhancement (`migrations/0002_coordination_and_calendar.sql`)
- Add `b3_trading_calendar` table storing official trading sessions and holiday schedules (1986–present).
- Add `b3_leases` table for durable worker task claiming and heartbeat fencing.
- Ensure rollback safety: down-migration drops only new tables without touching market facts.

### Step 2: Shared Query Services Layer (`src/services/query/`)
- Extract query logic duplicated between `src/api/routers/` and `src/mcp/tools.rs` into:
  - `src/services/query/instruments.rs`
  - `src/services/query/quotes.rs`
  - `src/services/query/indices.rs`
  - `src/services/query/governance.rs`
- Both REST handlers and MCP handlers become thin adapters calling these typed query services.

### Step 3: Hardened Acquisition and Streaming Decompression
- Update `B3DownloadService`: stream HTTP bodies directly to disk using `tokio::io::AsyncWriteExt` with bounded chunks, calculating SHA-256 in flight.
- Update `extract_archive_members`: stream entries from `ZipArchive` to temporary files or bounded buffers, returning explicit `AppError::Layout` when depth exceeds 4 or member size exceeds 500 MB.

### Step 4: Atomic Sync Execution & Session-Level Locks
- Refactor `B3SyncService`:
  - Acquire session-level advisory lock (`pg_advisory_lock`) on a dedicated checkout connection, holding it across the entire acquisition, parsing, and persistence phase.
  - Wrap domain facts insertion, audit records, and member status updates in an explicit, single PostgreSQL transaction.
  - Release lock predictably in an RAII drop guard or explicit `finally` block upon completion, failure, or cancellation.

### Step 5: B3 Trading Calendar & Scanner Decoupling in Worker
- Replace `Local::now()` and weekend checks in `src/worker/mod.rs` with:
  - Explicit `America/Sao_Paulo` timezone.
  - Query against `b3_trading_calendar` to verify if current date is a valid trading session.
  - Run discovery scanner (`b3_pending_updates`) first; only dispatch ingestion jobs when new artifacts are available.

### Step 6: MCP Server Independence
- In `src/main.rs`: Move `run_migrations(&pool)` into `Commands::Serve` and `Commands::Sync`, skipping migration execution for `Commands::Mcp`.
- Ensure MCP can connect with a read-only PostgreSQL role.

---

## 3. Shadow Comparison and Cutover Protocol

1. **Input Manifest**: Both legacy and enhanced paths receive the exact same raw files (e.g., `COTAHIST_A2026.TXT` and `IN260301.zip`).
2. **Fact Validation**: Verify that 100% of generated records in shadow execution match the baseline in:
   - Natural key components.
   - `stable_row_hash`.
   - Number of inserted vs changed records.
   - `b3_field_changes` audit records.
3. **Cutover Criteria**:
   - Zero discrepancy in business hashes.
   - Memory usage during annual COTAHIST ingestion remains under 256 MB.
   - Lock acquisition and release verified under concurrent load tests.

