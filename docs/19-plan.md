# Existing implementation review and migration plan

Status: needs-triage

Date: 2026-11-06, America/Sao_Paulo.

This plan implements the review and migration approach in [PRD section 19](b3_prd.md#19-aproveitamento-do-projeto-existente-e-migração). The [product PRD](b3_prd.md) remains the sole source of product requirements. This document defines how to assess existing code, preserve useful capabilities, and sequence remediation; it neither replaces the PRD nor declares its requirements implemented.

Agent communication, assessment reports, and this plan use English. Product documentation, API descriptions, errors, and other product-facing text remain Brazilian Portuguese unless explicitly requested otherwise. Preserve official source names and technical identifiers.

## 1. Scope and expected outcome

Review the existing Rust implementation against the entire PRD, using section 19 as the migration framework. Include acquisition, parsing, identity, persistence, auditing, coordination, REST, MCP, configuration, tests, CI, and operational documentation. Assess observable behavior and failure recovery, not merely the presence of modules or tables.

The outcome is a traceable assessment and an ordered implementation backlog that answers:

1. Which capabilities already satisfy the requirements, with what evidence?
2. Which code should be retained, extended, isolated behind a compatible adapter, or replaced?
3. Which gaps threaten financial correctness, audit history, identity, or availability?
4. What must migrate in data, contracts, and operations before switching consumers?
5. What evidence permits each migration step and eventual retirement of the old path?

Creating this plan does not initiate remediation, ingest market data, run migrations, create commits, or certify source access. Execution begins with the baseline and review phases below. External eligibility questions must not block independent offline review and deterministic testing.

## 2. Baseline and evidence rules

### 2.1 Capture the review baseline

At execution start, record the commit, branch, dirty working tree, PRD hash, dependency lockfile hash, Compose configuration identity, toolchain, and validation environment. Do not include resolved credentials or secret-bearing configuration in artifacts.

The preliminary inspection for this plan used HEAD `1e7ec9de97466376e85f37cdfad6a0615ed80641`. Documentation changes from the PRD promotion were already present. This is a reference point, not a claim that a clean commit represents all files inspected. Re-establish the baseline if implementation changes during the review.

Protect existing user changes. Use an isolated checkout/worktree when useful for experiments, and an isolated PostgreSQL 16 test database and raw directory. Never use an operational database for test cleanup or benchmark preparation.

### 2.2 Build the alignment matrix

Create `.scratch/prd-alignment/assessment.md` during execution, with one row per testable requirement. Do not create another PRD. Split broad claims such as “ingestion is idempotent” into repeat ingestion, retry, reparse, overlapping granularities, audit, timestamps, and recovery requirements.

Each row must include:

| Field | Required content |
| --- | --- |
| Requirement ID | Stable local identifier, such as `ING-001`, and exact PRD section |
| Invariant | Observable behavior or constraint being evaluated |
| Implementation | Repository path and symbol, plus line reference in the recorded baseline |
| Evidence | Static inspection, test name/result, query output, or runtime experiment |
| Alignment | `aligned`, `partial`, `missing`, `conflicting`, or `unknown` |
| Confidence | `static-only`, `runtime-verified`, or `blocked-by-evidence` |
| Risk | Severity, affected scope, and concrete failure consequence |
| Disposition | `retain`, `extend`, `replace`, `defer`, or `investigate` |
| Follow-up | Existing issue or newly scoped issue, dependencies, and acceptance evidence |

`Aligned` requires evidence appropriate to the requirement. Runtime invariants cannot be marked aligned using static inspection alone. A failed test is not automatically a product defect until environmental causes are distinguished. A test skipped because PostgreSQL is absent is not integration evidence.

Do not calculate an overall compliance percentage from arbitrary row counts. Summarize verified capabilities, blockers, and unresolved evidence by subsystem and release gate instead.

### 2.3 Prioritize findings

- **P0:** plausible loss/corruption of market facts or audit evidence, unsafe publication, credential exposure, or incompatible mutation of a published contract. Contain affected execution before migration proceeds.
- **P1:** correctness or recovery gap that blocks the relevant release gate, including ambiguous identity chosen as resolved, broken idempotency, ineffective cross-process coordination, or false operational success.
- **P2:** important boundedness, maintainability, documentation, or performance gap without a demonstrated immediate integrity failure. Escalate when evidence shows a stronger consequence.
- **P3:** improvement with no current acceptance-gate impact; explicitly defer when appropriate.

Severity follows the demonstrated scenario. Do not classify every absent target feature as an emergency.

## 3. Preliminary review leads to revalidate

These leads are based on static inspection while preparing this plan. They guide investigation; they are not a completed compliance audit.

| Area and current evidence | Review question and PRD mapping |
| --- | --- |
| [Downloader](../src/services/download_service.rs), `download`, materializes HTTP bodies with `resp.bytes().await?.to_vec()` | How are raw bytes persisted and recovered? Verify bounded acquisition, redirects, destination checks, and token redaction against sections 10.3–10.4 |
| [Archive extraction](../src/parsers/mod.rs), `extract_zip_recursive`, buffers entries and returns success above its depth limit | Can nested content be silently omitted? Verify inventory identity, decompression limits, and structural failure semantics |
| [Sync service](../src/services/sync_service.rs) retains a lock transaction while internal writes use the pool | Which writes actually share an atomic boundary? Preserve existing lock protection while testing data/audit/state rollback and lock lifetime separately |
| Sync URL builder sends the annual filename to the generic session-download route and lacks daily/monthly native branches | Which source/granularity capabilities are genuinely supported? Verify approved adapters and unsupported-source errors without assuming a working public endpoint |
| [Worker](../src/worker/mod.rs) uses `Local::now()`, weekend checks, and a repeating ingestion loop | How does this differ from discovery-first execution, B3 calendar, priority, and durable jobs in sections 10–11 and 17? |
| [Lock helper](../src/db/locks.rs), `try_acquire_pg_advisory_lock`, executes a transaction lock through a pool | Are any call sites relying on a lock after its transaction ends? Distinguish helper behavior from the transaction retained by the main sync flow |
| [COTAHIST parser](../src/parsers/cotahist.rs) returns collected rows and tolerates alternative trailer counts | Verify byte handling, encoding, structural rejection, quarantine isolation, and official counts independently |
| [Resolver](../src/db/resolver.rs) resolves security_id and gathers identifier candidates | Prove qualifier handling, precedence, contradictions, temporal windows, and late resolution against section 7 |
| [Database tests](../tests/upsert_and_audit_tests.rs) and other integration tests contain early returns when the database cannot be reached | Establish whether required scenarios truly execute in CI and Compose; fail the integration lane when its database is unavailable |
| [API/OpenAPI registration](../src/api/mod.rs) now lists substantially more handlers than the earlier inspection described | Reconcile actual routes, spec, schemas, auth, and behavior afresh; do not copy the old incomplete-registration claim as a current finding |
| [Entrypoint](../src/main.rs) runs migrations before dispatching commands | Determine how MCP can use a read-only database role without startup mutations; inspect query-service reuse in [MCP tools](../src/mcp/tools.rs) |

The presence of Decimal types, family-specific tables, or an audit helper is a reuse candidate, not proof that the full financial, family-isolation, or atomicity contract holds.

## 4. Review phases

### R0 — Freeze the evidence baseline and establish reliable checks

**Work:** capture section 2 metadata; read `PRODUCT.md`, `AGENTS.md`, applicable context documents/ADRs, the PRD, Compose, Dockerfiles, and CI. Record absent context files rather than creating empty placeholders. Verify the development service and isolate test resources.

Run the required checks from the repository root, recording exit codes, failures, and whether database-backed tests actually ran. Identify tests that can silently skip or use shared data. Record baseline failures before remediation so later changes are not credited with unrelated fixes.

**Deliverables:** baseline manifest, validation log, initial requirement matrix, and a list of unreliable or missing integration checks.

**Exit:** the team can distinguish a genuine integration run from a false green. If the database or Docker is unavailable, record the blocker and continue static work; leave runtime-dependent rows unknown.

### R1 — Inventory contracts and operational behavior

**Work:** enumerate every effective REST route, auth exception, handler, query parameter, DTO, error, and OpenAPI operation. Exercise representative authenticated/unauthenticated requests against isolated data. Capture generated OpenAPI and representative responses without secrets. Inventory CLI commands, worker behavior, and MCP tools/lifecycle.

Map all persisted domain and operational tables, constraints, timestamp fields, foreign keys, and provenance references. Trace one successful sync, duplicate, malformed record, failed member, retry, and identity rebuild through the code.

Compare README claims and operational commands with actual behavior. Investigate `/company-data`, pagination clamping, ordering tie-breakers, provenance, COTAHIST warnings, and the relationship between promised and implemented filters. Record known consumers; unknown consumers remain a compatibility risk rather than permission for breaking changes.

**Deliverables:** route/contract inventory, schema inventory, execution traces, and compatibility baseline.

**Exit:** each published surface has a baseline and a reuse or migration disposition. Spec presence alone does not establish behavioral parity.

### R2 — Review source acquisition, parsing, and raw recovery

**Work:** map allowed families and granularities to adapters, configuration, payload formats, parser versions, and raw storage. Link access/retention evidence from the [source research](RESEARCH.md); identify unresolved eligibility separately from implementation gaps. Do not repeat external research conclusions as newly verified permission.

Use deterministic HTTP mocks and authorized/synthetic fixtures to test acquisition, redirects, token expiry, interruption, false HTML success, archive limits, nested members, and replay after the remote source disappears. Verify that filenames and formats do not silently select an incorrect parser.

Review XML namespaces/paths, streaming, CSV/JSON profiles, repeated groups, fixed-width byte decoding, implied scales, sentinel dates, currency, factors, historical identifiers, and member/record provenance. Distinguish recoverable row errors from structural file failures.

Measure where full archives, members, row collections, and large records remain in memory. Record current resource behavior without publishing unsupported throughput claims.

**Deliverables:** capability map, fixture manifest, acquisition/parser evidence, and bounded-processing gaps.

**Exit:** each core source capability is either demonstrated or explicitly limited; raw/member recovery and structural failure behavior are understood. Unapproved live acquisition remains disabled.

### R3 — Prove persistence, identity, idempotency, and publication invariants

**Work:** use real PostgreSQL 16 to inspect and test every affected natural key, including nullable components, distribution and settlement-term discriminators. Profile collisions in permitted fixtures before proposing new constraints.

Test identical ingestion, one-field changes, equivalent decimal/string forms, duplicate members, retries, force reparse, and daily/monthly/annual overlap. Compare business fields, hashes, observation timestamps, change timestamps, field audit rows, provenance, and committed counters.

Inject failure between fact writes, audits, member state, and run completion. Determine whether partial work becomes visible and whether old valid results survive. Examine ignored write errors, replay semantics, replacement cleanup, absent rows in complete replacement files, and uncertain/partial scopes.

Test identifier changes/reuse, qualified IDs, ISIN/distribution collisions, missing historical evidence, out-of-order snapshots, ambiguity, and rebuild. Assert that relation changes do not change market values or their business hashes.

**Deliverables:** invariant test report, transaction-boundary map, identity decision table, and proposed migration requirements.

**Exit:** integrity-critical uncertainties are resolved or become explicit release blockers. Do not accept the new publication path before proving visibility and rollback boundaries.

### R4 — Review coordination, scheduling, and recovery

**Work:** test independent processes contending for an identical source scope and overlapping COTAHIST periods. Verify connection retention, rollback, cancellation, pool pressure, and lock release. Inspect durable claims, leases, fencing, heartbeats, retries, priority and cancellation; mark absent mechanisms as missing rather than emulating them with local mutexes.

Test market timezone, B3 holidays/special sessions, missing calendar evidence, maintenance on weekends, and publication windows. Distinguish scanner checks, downloads, parsing, promotion and identity jobs.

Evaluate run/member status transitions and counter correctness under mixed success, crashes and recovery. Check worker health, query isolation, administrative authorization, structured logs, metrics, and credential redaction.

**Deliverables:** concurrency/recovery scenarios, scheduler transition plan, and operational API gaps.

**Exit:** define the minimum coordination/control capabilities required alongside pipeline migration; do not defer recovery until after the new ingestion path is launched.

### R5 — Validate REST/MCP semantics and the consumer migration boundary

**Work:** run the same logical queries through REST and MCP and compare precision, selected family, identity, provenance, warning and coverage semantics. Inspect SQL duplication in transports and identify shared query-service extraction boundaries.

Test ambiguous series, multiple records per day, currencies/factors, empty results versus dependency failure, `include_internal`, pagination limits and ordering. Check whether current storage can support stable revision pagination and historical knowledge queries before exposing those capabilities.

Verify MCP with a compatible client, including initialization, tools/list, tools/call and errors. Exercise database read-only permissions and verify that startup does not require migration/write privileges. Do not infer protocol compliance from JSON-RPC responses alone.

**Deliverables:** contract parity report, query-service reuse map, compatibility classifications, and proposed additive/versioned changes.

**Exit:** every planned consumer-visible change has explicit before/after semantics, migration requirements, and contract tests.

### R6 — Consolidate findings into executable remediation

**Work:** complete all matrix rows; distinguish verified compliance, confirmed gaps, contradictory requirements, external blockers and missing evidence. Reconcile the preliminary leads with the actual results, including findings disproved by recent changes.

For each subsystem, choose retain/extend/replace with supporting evidence. Record B3-specific decisions where hard-to-reverse tradeoffs justify ADRs. Open or refine local issues, reusing the [existing delivery issues](issues/02-domain-and-contracts.md) where scopes overlap. Do not duplicate the backlog or alter requirements to match current shortcuts.

**Deliverables:** assessment report, reuse map, prioritized backlog, migration design and release-gate checklist.

**Exit:** each P0/P1 gap has a concrete scope, dependency, acceptance test, compatibility assessment and recovery plan. Unresolved decisions remain visible and block only dependent steps.

## 5. Apply section 19.1: reuse principles from tucano-cvm

Review these reference decisions during R2–R4:

| Reference | Evaluate for B3 | Do not import |
| --- | --- | --- |
| [ADR 0003: artifact-backed ingestion](../../tucano-cvm/docs/adr/0003-arquitetura-hibrida-de-artefatos-para-ingestao.md) | Recoverable raw, operational metadata and canonical PostgreSQL responsibilities | CVM-specific tables or an automatic object-storage dependency |
| [ADR 0004: typed direct path](../../tucano-cvm/docs/adr/0004-dfp-itr-direct-path-com-postgres-canonico.md) | Bounded normalization and typed staging where measurements justify it | DFP/ITR identity, document/version semantics or compulsory staging of every source |
| [ADR 0007: acknowledged equivalent remote baseline](../../tucano-cvm/docs/adr/0007-referencia-remota-reconhecida-sem-reingestao.md) | Separate changed packaging from changed member/business content | A claim that all B3 channels provide useful HTTP validators |
| [ADR 0008: publication versus observation](../../tucano-cvm/docs/adr/0008-radar-separa-fonte-publicacao-observacao.md) | Evidence-based publication time distinct from collection time | Editorial assumptions or timelines unrelated to trading data |

Revalidate the reference documents when executing the review. Their purpose is architectural comparison. Keep Rust, Tokio, Axum, SQLx, PostgreSQL 16 and the existing target stack; do not introduce Python runtime, Celery, Redis, SQLite-based integration substitutes, CVM enrichment or company identifiers into the B3 domain.

## 6. Apply section 19.3: implementation and migration sequence

Execute this sequence only after R6 has established the findings and designs. Retain compatible, proven behavior instead of scheduling a wholesale rewrite.

| PRD step | Implementation package | Completion evidence |
| --- | --- | --- |
| 1. Inventory contracts and consumers | Preserve the R1/R5 baseline and add characterization tests for affected behavior | Saved spec/responses, known-consumer inventory and compatibility tests |
| 2. Record decisions and unresolved evidence | Specify identity qualifiers/windows, source capabilities, grain, observations, revisions and selection rules | B3 decisions linked to requirements, unresolved external dependencies explicit |
| 3. Add migrations | Use additive schema changes and scoped backfills; preserve IDs, facts and audit | Empty-database and previous-version upgrade tests; counts and references reconciled |
| 4. Implement a small parallel path | Separate source adapters, artifact handling, parsing, persistence and shared queries; start with a bounded COTAHIST scope and available identity evidence | Shadow comparison without duplicate publication; bounded resource measurements |
| 5. Publish a validated revision | Validate all required members and switch visibility atomically for the selected scope | Readers never see an unfinished revision; prior valid revision remains recoverable |
| 6. Migrate queries and worker | Move each capability behind compatible adapters or an explicit version transition; add durable coordination before enabling new jobs | REST/MCP parity, controlled worker cutover, concurrency/recovery and smoke results |
| 7. Expand and retire | Grow historical scope and source families only after integrity/resource gates pass; remove old paths when parity is proven | Restore rehearsal, resolved differences, consumer transition and cleanup evidence |

### 6.1 Data migration safeguards

- Do not edit the applied [initial migration](../migrations/0001_initial_b3_schema.sql). Plan new SQLx migrations and validate upgrade from the recorded schema baseline.
- Inventory available raw bytes and provenance before backfill. Missing evidence remains unknown; do not fabricate publication timestamps, historical identity windows, or raw hashes.
- Preserve source observations and business history while separating facts, identity links and selected revisions. Do not deduplicate different market facts merely because their tickers match.
- Dry-run backfills on an isolated copy; reconcile row/key counts, nullable uniqueness, foreign keys, audit continuity and selected values. Counts alone are insufficient: compare canonical business content by family/grain.
- Make long backfills bounded and restartable. Specify checkpoints, idempotency keys and re-entry behavior before running them.
- Gate any reconciliation/deletion on a verified complete replacement scope. Preserve prior versions and explain withdrawals.

### 6.2 Shadow comparison and cutover

Feed both paths from the same permitted immutable input and rule-version manifest. Compare normalized facts by agreed natural key, selected revision, identity outcome, timestamps, audit and counters. Label intentional semantic corrections separately from regressions. An old implementation's defect is not a behavior to reproduce for parity.

Run shadow processing in isolated staging/storage namespaces; only one path publishes. Measure resource contention and ensure shadow/backfill work cannot starve maintenance. The cutover checklist must identify selected family/period, application/schema versions, active jobs, compatible consumers, rollback pointer and responsible operator.

Stop new claims for the affected scope, drain or safely cancel active work, verify fencing/locks, and switch only a validated scope. Observe errors, queue lag, unresolved/conflict counts, query behavior and persisted counters before expanding.

### 6.3 Recovery and rollback

Rehearse a consistent PostgreSQL-plus-raw restore before production cutover. Keep the prior valid revision and compatible application path available for the agreed transition period. Define rollback triggers from integrity failures and operational budgets established by measurement.

Rollback selects a known valid revision/application compatible with the expanded schema and stops incompatible jobs. It does not erase new audit history or assume destructive down-migrations are safe. Restoring a backup after new writes requires reconciliation of those writes and an explicit recovery decision; it is not a routine invisible rollback.

Retire old paths only when accepted corrections are documented, consumers have migrated, rollback/restore has been demonstrated, and retention rules protect referenced evidence.

## 7. Execution dependencies and release gates

Review order: `R0 → R1 → R2 → R3 → R4 → R5 → R6`. Individual static reads may proceed independently, but runtime conclusions depend on R0's reliable environment and comparison depends on R1's baseline. External source eligibility gates live acquisition/public distribution, not offline assessment.

Map remediation onto PRD G0–G7 rather than inventing a second roadmap:

- G0: source eligibility and permitted corpus.
- G1: contract, identity, grain and revision decisions.
- G2/G3/G5: acquisition, historical core, atomicity and operational recovery developed together.
- G4: validated query services and compatible REST/MCP behavior.
- G6/G7: family-specific price reports, indices and portfolios after common integrity gates.

Required gates:

| Gate | Blocking evidence |
| --- | --- |
| Assessment complete | Requirements mapped; no unexplained P0/P1 findings; runtime unknowns identified |
| Schema migration ready | Upgrade/empty initialization, preserved history, recoverable scoped backfill |
| First scope publishable | Permitted input, precision, grain, temporal identity, audit, counters and failure visibility proven |
| Worker cutover ready | Cross-process locks, overlap, lease/fencing where applicable, cancellation, calendar and reserved capacity proven |
| Consumer cutover ready | OpenAPI/runtime parity, read-only MCP, migration windows and public limitations documented |
| Expansion/retirement ready | Measured resource budget, accepted shadow differences, restore/rollback rehearsal and consumer transition complete |

Do not promise duration or throughput before the assessment identifies corpus, dependencies and the first measured baseline.

## 8. Validation protocol

Run Rust/dependency/migration checks through the Compose development service:

```bash
docker compose config --quiet
docker compose run --rm cargo cargo fmt --all -- --check
docker compose run --rm cargo cargo clippy --all-targets --all-features -- -D warnings
docker compose run --rm cargo cargo test --all-targets --all-features
```

`cargo` must connect to PostgreSQL 16 through `DATABASE_URL`. Record actual test execution, including any early returns. A green aggregate result cannot override a missing integration dependency.

For runtime/startup changes, also build and smoke-test affected components:

```bash
docker compose build b3_api b3_worker
```

Inspect current service configuration before starting processes. The current Compose defaults enable synchronization; prepare acquisition-disabled or deterministic fixture configuration before starting the worker. Verify readiness, worker liveness, logs, cancellation and restart behavior. Recheck the current host mappings (API `8010`, PostgreSQL `5433`) and internal ports rather than copying assumptions into commands.

CI currently uses a native Rust toolchain with PostgreSQL 16 and a Docker image build. Compare its flags and database semantics with Compose; log differences and fix relevant drift alongside infrastructure changes. CI image build alone does not prove API/worker startup.

Use deterministic external HTTP mocks in tests. Live source tests are separate, permitted, opt-in and narrowly scoped. Benchmarks identify code/rule versions, hardware, storage, PostgreSQL settings, fixture hashes, compression sizes, batch limits and concurrency; separate parsing from persistence and query load.

Documentation-only changes require local links/anchors, commands, examples and whitespace checks. Run a documentation build only if a configured generator is affected. Report executed, failed, skipped and unavailable validations with reasons.

## 9. Review artifacts and implementation handoff

Create populated artifacts under `.scratch/prd-alignment/` as execution produces evidence:

- `assessment.md`: requirement matrix, severity-ranked findings and subsystem summary.
- `evidence/`: baseline manifest, sanitized OpenAPI/responses, validation results, fixture manifests and comparison reports. Do not commit credentials or raw market files without compatible rights.
- `migration.md`: selected designs, additive schema/backfill plan, cutover and rollback checklist.
Accepted remediation issues belong in `docs/issues/<NN>-<slug>.md`, using the next available number and the [backlog index](issues/README.md). Keep only drafts under `.scratch/prd-alignment/`; avoid duplicating existing issues. Use project triage statuses, dependencies, acceptance evidence and `## Comments`.

Each implementation slice should include its regression tests, migration and related documentation. Use focused commits following the repository's existing convention, for example `fix(ingestion): ...` or `test(identity): ...`. Do not commit unrelated working-tree changes or describe a feature as verified when evidence is incomplete.

Update product-facing OpenAPI, README, operational documentation and `docs/frontend_api_changelog.md` when implementation changes public behavior. This plan itself changes no API contract and needs no consumer API changelog entry.

Final review delivery must state: what was retained; what was replaced and why; verified PRD alignment; remaining unknowns; compatibility/data risks; exact checks and database scenarios executed; and the next executable slice. Plans and unexecuted tests are not evidence of compliance.

## 10. Completion criteria for this plan document

- Section 19.1 reuse principles, 19.2 review leads and all seven 19.3 migration steps are covered.
- The plan distinguishes current static observations from future tests and verified behavior.
- All PRD subsystems have a review path, evidence model and release gate.
- Migration preserves contracts, IDs, raw provenance, temporal identity, market facts and audit history.
- Source access uncertainty and unavailable runtime checks remain explicit.
- This document is English; the product PRD remains authoritative and product communication remains pt-BR.

## Comments

- 2026-11-06 — Requested an English development plan in `docs/` to review existing code against the adopted PRD, specifically section 19. Preliminary code inspection informed the review leads; no implementation, runtime validation or migration was performed while authoring this plan.

- 2026-11-06 — Source research, domain glossary and delivery backlog promoted to permanent locations. [Issue 09](issues/09-prd-alignment-review.md) is the executable entry point for this plan.
