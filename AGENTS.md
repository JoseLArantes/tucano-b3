# AGENTS.md

`tucano-b3` is a service for ingesting, normalizing, storing, and exposing official public B3 data through a REST API and a read-only analytical MCP server.

Target stack: Rust, Tokio, Axum, SQLx, PostgreSQL 16, quick-xml, rust_decimal, and Utoipa/OpenAPI 3.1. The API, worker, and MCP share the same service library; coordination and queues use PostgreSQL.

This file guides the project's intended evolution. The rules below are requirements, not a claim that every capability already exists. `tucano-cvm` is the reference for architecture, documentation, and organization; domain and technology decisions belong to `tucano-b3`.

## Working rules and sources of truth

- Read `PRODUCT.md` for product purpose and principles, and `docs/b3_prd.md` before changing flows or contracts. The PRD defines scope and target behavior.
- Read the existing implementation, tests, and contracts to understand current behavior and the impact of a change. Current gaps do not redefine product requirements.
- Consult `CONTEXT-MAP.md`, relevant `CONTEXT.md` files, and `docs/adr/` when present. Surface conflicts with recorded decisions; do not resolve them silently.
- Preserve published contracts or explicitly plan their migration. Identify discrepancies between the PRD, README, and code, and address them within the task's scope.
- Fix root causes and maintain module boundaries. Do not perpetuate shortcuts merely because they already exist or make unrelated refactors.
- Use `rg` for search and `apply_patch` for manual edits. Do not revert user changes or use destructive Git commands.
- Keep `AGENTS.md` and `PRODUCT.md` in English. Write product documentation and user-facing text in Brazilian Portuguese unless the task explicitly requires otherwise. Preserve technical identifiers and official source names.
- Operational and consumer documentation describes implemented, verified behavior. PRDs, ADRs, and plans may describe the target state while identifying pending requirements.
- PRD performance targets remain targets until measured. Do not publish latency, memory, or throughput figures as results without reproducible evidence.

## Required validation

For changes to Rust code, dependencies, or migrations, run these commands through the Compose development service:

```bash
docker compose run --rm cargo cargo fmt --all -- --check
docker compose run --rm cargo cargo clippy --all-targets --all-features -- -D warnings
docker compose run --rm cargo cargo test --all-targets --all-features
```

- Keep these commands aligned with `docker-compose.yml` and CI when infrastructure changes.
- Persistence, migration, concurrency, and lock tests require PostgreSQL 16 accessible through `DATABASE_URL`. A test that returns early because the database is unavailable does not validate the integration.
- Runtime, Docker, configuration, or startup changes require a build and smoke test of the API and worker. Check actual Compose ports, services, and commands before running them.
- Documentation-only changes require reviewing consistency, links, paths, and commands. Run the documentation build when a configured site or generator is affected.
- At delivery, report what was validated, what was not run, and why. Do not claim success for skipped or unavailable checks.

## Architectural map and boundaries

- Entrypoints and configuration: `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/error.rs`.
- REST and contracts: `src/api/`, `src/api/routers/`, `src/api/schemas.rs`.
- Persistence, identity, and auditing: `src/db/`, `migrations/`.
- Parsing and canonicalization: `src/parsers/`.
- Acquisition and orchestration: `src/services/`.
- Background execution: `src/worker/`.
- Analytical MCP: `src/mcp/`.
- Tests: `tests/`. Local runtime: `Dockerfile`, `Dockerfile.dev`, `docker-compose.yml`.

Routes, MCP tools, and CLI commands are thin adapters over shared services. Do not duplicate business rules across transports. Parsers interpret files and produce typed records; they do not download files or control transactions. Services orchestrate acquisition and ingestion; the persistence layer owns SQL, upserts, locks, and persisted relationship resolution.

Split modules by responsibility and source family as they grow. Avoid concentrating new flows in monolithic files or introducing generic abstractions that erase differences between sources.

Every persisted structural change requires a new SQLx migration in `migrations/`. Do not rewrite migrations that have already been applied. Validate both empty-database initialization and upgrades from the previous version.

## Domain scope and identity

- Accept only official public B3 sources: `BVBG.028.02`/`InstrumentsConsolidated`, Tradable Security List, `BVBG.086.01`, `BVBG.186.01`, `BVBG.087.01`, index portfolio compositions, and daily, monthly, and annual COTAHIST.
- CVM data, external enrichment, broker feeds, and unofficial sources are outside this product's scope. `CNPJ_CIA`, `CD_CVM`, and external identifiers are not relationship anchors.
- The B3 instrument registry is the canonical identity root. Prioritize its ingestion before dependent sources while preserving historical records that cannot yet be resolved.
- Resolve by `security_id`, then by ISIN, ticker, market, and date according to PRD precedence and `valid_from`/`valid_to` windows. A ticker alone is not a stable identity.
- Reuse unchanged identifier windows and close superseded windows deterministically. Do not apply current identity retroactively to all history.
- Preserve `unresolved` and `ambiguous` states; do not pick an instrument arbitrarily. Identity rebuilds must be idempotent, support late resolution, and preserve market values and their provenance.
- Keep `BVBG.086` and `BVBG.186` separate in persistence, contracts, and lineage. Comparisons must be explicit; do not silently merge or fall back across families.

## Ingestion and operations

- Ingestion is a sensitive, priority pipeline. Analytical queries, backfills, and rebuilds must not exhaust resources needed for regular updates.
- Separate lightweight update discovery (`b3_pending_updates`) from downloads and heavy processing.
- Preserve two phases: acquisition, integrity checks, and member inventory; then streaming parsing, resolution, and persistence in bounded batches.
- Record immutable raw files, SHA-256, size, parser version, and all nested ZIP members in deterministic order. Preserve provenance down to the member and line/record index.
- Validate official hosts, including redirects and two-step downloads. Protect extraction against path traversal and enforce decompression, nesting-depth, and memory limits.
- Use `b3_sync_runs`, `b3_source_files`, and `b3_source_file_members` as operational state sources. Counters, failures, and transitions must reflect work actually persisted.
- Repetition, retry, and `force_reparse` must not duplicate records, erase valid audit history, or produce phantom changes. Cleanup and replacement of reprocessed state must be transactional within the affected scope and recoverable after failure.
- Invalid records go to `b3_quarantine_records`, with a reason and location, allowing replay without losing valid records. File and infrastructure failures remain visible; do not turn them into success or empty datasets.
- Coordinate the API, CLI, scheduler, and worker through the same PostgreSQL locks and states. Retain the connection/transaction holding a lock throughout the protected section and release resources on failure or cancellation.
- Reject concurrent attempts for the same source/date predictably; use `409 Conflict` in the HTTP contract. Do not introduce Redis, Celery, or memory-only queues as durable coordination.
- The scheduler uses the B3 trading calendar and `America/Sao_Paulo`; weekend checks alone do not identify trading sessions. Reference dates are market dates; persisted timestamps are UTC.
- Expose health, coverage, pending updates, quarantine, and statistics by run, file, and member. Structured logs must support correlating these entities without exposing credentials or temporary tokens.

## Normalization, idempotency, and auditing

- Use `rust_decimal::Decimal` for financial values and decimal quantities. Never convert through `f32`/`f64`; scales, currency, quotation factors, and units must be explicit.
- Canonicalize strings, tickers, ISINs, sentinel dates, and decimals before comparison and hashing. Business hashes must be deterministic and independent of field order and operational metadata.
- Preserve source-specific natural keys and uniqueness with `NULLS NOT DISTINCT` when key components are nullable. `DISMES`/`distribution_number` is part of the COTAHIST key; `days_to_settlement` distinguishes forward-market PriceReport records.
- Separate `created_at`, `last_seen_at`, `data_changed_at`, and `field_changed_at`. Observing a record again does not mean changing its business content.
- A canonically identical row must not change business fields, its hash, `data_changed_at`, or `field_changed_at`, or emit `b3_field_changes`. Operational observation updates follow their own contract.
- A change updates only the business fields actually modified and their timestamps, emitting old/new value audit records in `b3_field_changes` within the same transaction.
- Prefer batched upserts and set-based SQL. Preserve atomicity across data, audit records, and operational state; avoid per-record queries and loading entire files into memory.

## REST and MCP contracts

- Use `/api/v1/b3/...` for the B3 domain and group endpoints, schemas, and OpenAPI tags by file family according to the PRD.
- Public contracts must be closed and typed. Do not expose internal structures or arbitrary payloads as substitutes for domain DTOs.
- Apply Bearer authentication to protected routes and keep public exceptions explicit and tested. Do not extend a health or documentation exception to ingestion operations.
- List endpoints require documented filters, `limit` defaulting to 100 with a maximum of 1000, `offset`, and deterministic ordering with a unique tie-breaker.
- Serialize decimals as strings and dates in ISO format. Every item must expose `provenance`; additional internal metadata is conditional on `include_internal=true`.
- COTAHIST exposes raw nominal historical prices without retroactive corporate-action adjustments. Preserve this warning in payloads, OpenAPI, and consumer documentation. Do not present derived series as officially published data.
- MCP is analytical and read-only. Reuse query services and the same precision, identity, and provenance rules as REST; do not expose sync, replay, rebuild, or other mutations as MCP tools.
- Do not run ingestion or heavy reprocessing during queries. Administrative operations require their own authorization and execution tracking.

## Testing rules

- Cover observable invariants and regressions, including failure and recovery, without mirroring the implementation in assertions.
- Test parsers against official ISO 20022 structures, namespaces, and representative fixtures as well as simplified examples. Cover archives with multiple members and nested files.
- For COTAHIST, test byte width, encoding, headers/trailers, implied decimals, sentinel dates, `FATCOT`, `DISMES`, and isolation of invalid lines.
- Prove idempotency for repeated ingestion, retry, and reparse; single-field changes; stable timestamps; atomic auditing; and natural keys containing `NULL`.
- Test temporal windows, ticker changes/reuse, ambiguous collisions, out-of-order ingestion, and retroactive rebuilds without changes to market values.
- Use real PostgreSQL for JSONB, uniqueness, transaction, and lock semantics. Isolate test data; do not depend on an operational database or replace these guarantees with SQLite or mocks.
- Public changes require REST and OpenAPI tests for authentication, schemas, filters, pagination, provenance, internal metadata, and the COTAHIST warning. MCP changes also require checking read-only behavior.
- Worker or ingestion changes require concurrency, calendar, cancellation, partial-failure, status, and counter scenarios. Mock external HTTP in deterministic tests; live acquisition checks must be explicit and separate.

## Required documentation

When a change affects endpoints, payloads, filters, fields, configuration, or operational flows, update the corresponding artifacts in the same delivery:

- Utoipa-generated OpenAPI, including detailed descriptions of fields, units, sources, parameters, examples, errors, authentication, and limitations.
- README and consumer/operational documentation in `docs/`, reflecting behavior actually available.
- `docs/frontend_api_changelog.md` for consumer-visible changes. Create it on the first applicable change; record the correct session date, endpoints, fields, and compatibility impact.
- `.env.example`, Compose, and runtime instructions when environment variables or operational dependencies change.
- The PRD, context documents, and ADRs when a decision changes scope, vocabulary, or target architecture. Do not weaken requirements to justify an incomplete implementation.

Do not document planned capabilities as delivered or copy Python/Celery/Redis instructions from the reference project. New pages must clearly distinguish public contracts, operations, and architectural decisions.

## Work organization and domain documentation

- Feature issues and PRDs use local Markdown under `.scratch/<feature-slug>/`; `docs/b3_prd.md` remains the overall product PRD. Do not use external PRs as the default triage surface.
- When a task requires traceable planning, use `PRD.md` and `issues/<NN>-<slug>.md`, numbered from `01`, with `Status:` near the top and conversation history under `## Comments`.
- Triage vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`.
- Domain documentation follows a multi-context layout: root `CONTEXT-MAP.md` points to each context's `CONTEXT.md`; global decisions live in `docs/adr/`, and local decisions live alongside the context as indicated in the map.
- Use context vocabulary in code, tests, and proposals. Record new definitions and decisions when resolved; do not create empty documents just to fill the structure. If these files do not yet exist, proceed with the PRD.

## Delivery checklist

1. Confirm alignment with the PRD, domain boundaries, and relevant architectural decisions.
2. Preserve temporal identity, provenance, idempotency, auditing, and compatibility of affected contracts.
3. Add a migration and regression tests when applicable.
4. Run formatting, Clippy, and tests; confirm required integration tests actually used PostgreSQL.
5. Update OpenAPI, consumer/operational documentation, and the changelog when applicable.
6. Validate builds and execution of affected components; report results and remaining gaps without confusing intent with implementation.
