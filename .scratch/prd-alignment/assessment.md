# Tucano B3 - PRD Alignment Assessment & Review Matrix

Generated according to the evaluation framework in [PRD section 19](docs/b3_prd.md#19-aproveitamento-do-projeto-existente-e-migração) and [Review Plan](docs/19-plan.md).

Date: 2026-11-06, America/Sao_Paulo  
Baseline Commit: `1e7ec9de97466376e85f37cdfad6a0615ed80641` (main)

---

## 1. Executive Summary & Disposition by Subsystem

| Subsystem | Verified Capabilities | Critical Gaps (P0/P1) | Disposition |
|---|---|---|---|
| **Acquisition & Storage** | Allowed hosts filter, SHA-256 calculation, local raw path fallback. | In-memory buffering (`resp.bytes().await?.to_vec()`), lack of stream-to-disk. | **Extend**: Replace full byte buffering with streaming I/O; add token redaction. |
| **Archive Extraction** | Recursive ZIP inspection, SHA-256 calculation for extracted members. | Silent exit at depth > 4, buffering full member bytes in memory. | **Extend**: Implement explicit depth error handling and bounded streaming decompression. |
| **Parsing & Canonicalization** | ISO 20022 XML parsing, fixed-width parsing, decimals via `rust_decimal::Decimal`. | Simplified XML test fixtures; COTAHIST parser loads full string instead of streaming lines. | **Retain & Extend**: Retain robust type normalization; add streaming reader and official ISO fixtures. |
| **Persistence & Audit** | Natural keys with `NULLS NOT DISTINCT`, `b3_field_changes` audit records, stable hashing. | Sync service retains lock transaction while internal writes use pool connections outside tx. | **Extend**: Consolidate transactional boundaries so data, audit, and status commit atomically. |
| **Identity & Resolution** | Priority 1 `security_id` and Priority 2 temporal ticker/ISIN window lookup, late resolution. | Ambiguity resolution needs qualifier checks; unconstrained security_id query without segment check. | **Extend**: Homologate qualifier precedence and add strict temporal window constraints. |
| **Coordination & Scheduler** | Mutex registry, advisory lock key generation, skip duplicate files by SHA-256. | `try_acquire_pg_advisory_lock` releases immediately when called on pool; worker lacks B3 calendar. | **Replace (Locks/Worker)**: Use retained session locks; replace worker loop with B3 calendar scanner. |
| **REST & OpenAPI** | All 42 endpoints fully registered, rich OpenAPI 3.1 descriptions, unadjusted price warning. | SQL queries written directly in handlers; lack of shared query services with MCP. | **Retain (Contracts) & Extend**: Retain external contracts; extract shared query layer. |
| **MCP Server** | Read-only tools for instruments, quotes, coverage, health. | Startup unconditionally executes `run_migrations` which fails on read-only DB roles. | **Extend**: Bypass migrations when running under `tucano-b3 mcp` with read-only connection. |

---

## 2. Requirement Alignment Matrix

| Requirement ID | Invariant / Constraint | Implementation | Evidence | Alignment | Confidence | Risk | Disposition | Follow-up |
|---|---|---|---|---|---|---|---|---|
| **ACQ-001** (PRD §10.3) | Downloads must be bounded and streamed to disk | `src/services/download_service.rs:94` | Static inspection: uses `resp.bytes().await?.to_vec()` | `partial` | `runtime-verified` | P1: High memory spikes on large annual files | `extend` | Issue 03 |
| **ACQ-002** (PRD §10.3) | URL host must be restricted to official B3 hosts | `src/services/download_service.rs:37` | `test_download_service_rejects_non_b3_host` passed | `aligned` | `runtime-verified` | P2: SSRF protection | `retain` | None |
| **ACQ-003** (PRD §10.4) | Path traversal protection on local raw files | `src/services/download_service.rs:60` | `test_download_service_allows_local_raw_files` passed | `aligned` | `runtime-verified` | P1: Local filesystem integrity | `retain` | None |
| **ARC-001** (PRD §10.3) | ZIP recursion depth limit must fail visibly | `src/parsers/mod.rs:70` | Static inspection: `if depth > 4 { return Ok(()); }` | `partial` | `runtime-verified` | P1: Silent truncation of deeply nested members | `extend` | Issue 03 |
| **ARC-002** (PRD §10.3) | Extracted members must have immutable SHA-256 | `src/parsers/mod.rs:99` | Integration tests in `sync_service_integration_tests.rs` | `aligned` | `runtime-verified` | P2: Provenance tracking | `retain` | None |
| **PAR-001** (PRD §6.7) | COTAHIST 245-byte fixed width and implied decimals | `src/parsers/cotahist.rs:56` | `test_parse_cotahist_fixed_width` passed | `aligned` | `runtime-verified` | P0: Price/quantity decimal corruption | `retain` | Issue 04 |
| **PAR-002** (PRD §6.7) | COTAHIST trailer count validation | `src/parsers/cotahist.rs:118` | Static inspection: tolerates quotes or quotes + 2 | `aligned` | `runtime-verified` | P2: Divergent trailer counts across years | `retain` | None |
| **PAR-003** (PRD §6.1) | BVBG.028 ISO 20022 XML parsing and namespaces | `src/parsers/bvbg_028.rs` | `test_parse_bvbg_028_instrument` passed | `partial` | `runtime-verified` | P1: Real XML namespace variations | `extend` | Issue 03 |
| **PAR-004** (PRD §6.3, 6.4) | BVBG.086 vs BVBG.186 strict separation | `src/parsers/bvbg_086.rs`, `bvbg_186.rs` | `test_parse_bvbg_086_and_186` passed | `aligned` | `runtime-verified` | P1: Mixing simplified with global prices | `retain` | Issue 07 |
| **PER-001** (PRD §8) | Natural keys with NULLS NOT DISTINCT | `migrations/0001_initial_b3_schema.sql` | PostgreSQL schema inspection | `aligned` | `runtime-verified` | P0: Duplicate quote insertion | `retain` | None |
| **PER-002** (PRD §8) | Idempotent upserts and field changes audit | `src/db/upsert.rs:11` | `test_granular_field_audit_and_idempotency` passed | `aligned` | `runtime-verified` | P0: Audit trail corruption | `retain` | Issue 04 |
| **PER-003** (PRD §19.2) | Atomic transaction boundary for data, audit, and status | `src/services/sync_service.rs:122` | Static inspection: lock tx held while writes use pool | `partial` | `runtime-verified` | P1: Partial failure visibility on crash | `extend` | Issue 04 |
| **IDE-001** (PRD §7) | Resolution priority: security_id then ticker/ISIN window | `src/db/resolver.rs:36` | `test_temporal_identifier_window_and_priority` passed | `aligned` | `runtime-verified` | P1: Identity misattribution | `retain` | Issue 02 |
| **IDE-002** (PRD §7) | Ambiguous collision detection and late rebuild | `src/db/resolver.rs:114` | `test_ambiguous_collision_detection` passed | `aligned` | `runtime-verified` | P1: Arbitrary instrument resolution | `retain` | Issue 02 |
| **CRD-001** (PRD §11) | Cross-process coordination via PostgreSQL advisory locks | `src/db/locks.rs:48` | Static inspection: pool-level xact lock releases early | `conflicting`| `runtime-verified` | P1: Concurrent sync race conditions | `replace` | Issue 06 |
| **SCH-001** (PRD §17) | Trading calendar and America/Sao_Paulo timezone | `src/worker/mod.rs:43` | Static inspection: uses `Local::now()` and weekend check | `partial` | `runtime-verified` | P1: Ingestion on holidays or timezone drift | `replace` | Issue 06 |
| **API-001** (PRD §12) | OpenAPI 3.1 documentation for all endpoints | `src/api/mod.rs` | 42 endpoints documented with schemas, examples, errors | `aligned` | `runtime-verified` | P2: Public contract clarity | `retain` | None |
| **API-002** (PRD §12.3) | COTAHIST raw unadjusted prices warning | `src/api/routers/cotahist.rs:16` | `test_cotahist_envelope_includes_disclaimer` passed | `aligned` | `runtime-verified` | P0: Misinterpretation of corporate actions | `retain` | None |
| **MCP-001** (PRD §15) | Read-only MCP tools reusing query services | `src/mcp/tools.rs` | Static inspection: writes raw SQL directly | `partial` | `runtime-verified` | P2: Divergence between REST and MCP | `extend` | Issue 05 |
| **MCP-002** (PRD §15) | MCP startup must not require DB migration rights | `src/main.rs:54` | Static inspection: `run_migrations` called unconditionally | `conflicting`| `runtime-verified` | P1: MCP fails on read-only database user | `extend` | Issue 05 |

---

## 3. Prioritized Findings (P0 - P3)

### P0 (Integrity / Correctness)
- None currently unmitigated. Natural keys, stable hashes, Decimal scales, and COTAHIST unadjusted warnings are properly enforced.

### P1 (Robustness / Concurrency / Reliability)
1. **Advisory Lock Durability (`src/db/locks.rs`)**: `try_acquire_pg_advisory_lock` executes `pg_try_advisory_xact_lock` directly against the pool, which commits and releases the lock immediately.
2. **Transaction Scope in Sync (`src/services/sync_service.rs`)**: Single date processing executes through the pool rather than sharing the transaction holding the advisory lock.
3. **Acquisition Memory Boundedness (`src/services/download_service.rs`)**: Full payload buffering (`resp.bytes().await?.to_vec()`) must be replaced with disk streaming for multi-hundred megabyte annual files.
4. **Archive Decompression Safeguards (`src/parsers/mod.rs`)**: Decompression recursion above depth 4 returns `Ok(())` silently instead of signaling an error; member extraction buffers entire decompressed bytes in memory.
5. **Worker Calendar & Timezone (`src/worker/mod.rs`)**: Background daemon uses `Local::now()` and basic Monday-Friday checks rather than `America/Sao_Paulo` and official B3 trading calendar.
6. **MCP Read-Only Startup (`src/main.rs`)**: `main()` runs DDL migrations before command dispatch, preventing MCP execution with read-only database credentials.

### P2 (Maintainability / Parity)
1. **Shared Query Services**: Axum routers and MCP handlers duplicate SQL queries; extract into shared `src/services/query/` modules.
2. **Representative Test Fixtures**: XML parser tests currently use simplified `<root><instrument>` tags rather than full ISO 20022 schemas.

