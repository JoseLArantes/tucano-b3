# AGENTS.md

`tucano-b3` is a standalone Rust service that ingests official public B3 market-data files (COTAHIST and IBOV), stores them as year-partitioned Parquet plus an embedded SQLite ledger, and serves them through a high-performance REST API with OpenAPI, auth, caching, rate limiting, and observability.

Target stack: Rust, Tokio, Axum, Apache DataFusion / Arrow / Parquet, rusqlite, DashMap/LRU cache, utoipa/OpenAPI, Prometheus, and tracing. The Docusaurus site in `docusaurus/` is the detailed public and operational documentation surface for this project.

This file guides agent work against the shipped product. Read `PRODUCT.md` for purpose, scope, and principles before changing flows or contracts.

## Working rules and sources of truth

- Read `PRODUCT.md` for product purpose, scope boundaries, design principles, and non-negotiable invariants.
- Read the existing implementation, tests, and contracts to understand current behavior and the impact of a change. Current gaps do not redefine product requirements; revise `PRODUCT.md` explicitly when purpose or scope must change.
- Consult `CONTEXT-MAP.md`, relevant `CONTEXT.md` files, and `docs/adr/` when present. Surface conflicts with recorded decisions; do not resolve them silently.
- Preserve published contracts or explicitly plan their migration. Identify discrepancies between `PRODUCT.md`, README, OpenAPI, and code, and address them within the task's scope.
- Fix root causes and maintain module boundaries. Do not perpetuate shortcuts merely because they already exist or make unrelated refactors.
- Use `rg` for search and `apply_patch` for manual edits. Do not revert user changes or use destructive Git commands.
- Keep `AGENTS.md` and `PRODUCT.md` in English. Agent communication remains in English. Write product documentation and product-facing text in Brazilian Portuguese unless the task explicitly requires otherwise. Preserve technical identifiers and official source names.
- Operational and consumer documentation describes implemented, verified behavior. ADRs and plans may describe target state while identifying pending requirements.
- Performance targets remain targets until measured. Do not publish latency, memory, or throughput figures as results without reproducible evidence.

## Required validation

For changes to Rust code, dependencies, or data schemas, run these commands through the Compose cargo service (or equivalent local Cargo when Compose is unavailable — report which path was used):

```bash
docker compose run --rm cargo cargo fmt --all -- --check
docker compose run --rm cargo cargo clippy --all-targets --all-features -- -D warnings
docker compose run --rm cargo cargo test --all-targets --all-features
```

- Keep these commands aligned with `docker-compose.yml` and CI when infrastructure changes.
- Ingestion, ledger, Parquet schema, and query tests must exercise real fixtures and the embedded SQLite ledger where those paths are under change. A test that returns early because data is unavailable does not validate the integration.
- Runtime, Docker, configuration, or startup changes require a build and smoke test of the API. Check actual Compose ports, services, and commands before running them.
- Documentation-only changes require reviewing consistency, links, paths, and commands. Run `npm --prefix docusaurus run build` when Docusaurus content, configuration, or its generator is affected.
- At delivery, report what was validated, what was not run, and why. Do not claim success for skipped or unavailable checks.

## Architectural map and boundaries

- Entrypoints and configuration: `src/main.rs`, `src/lib.rs`, `src/config/`, `src/error.rs`.
- REST and contracts: `src/api/`, `src/models/`.
- Query and columnar data: `src/data/`, `src/services/`.
- Ingestion and parsing: `src/ingest/`.
- Ledger and ticker registry: `src/ledger/`, `src/data/ticker_registry.rs`.
- Cache: `src/cache/`.
- Middleware (auth, rate limit, metrics, request id): `src/middleware/`.
- Observability: `src/metrics/`.
- Tests: `tests/`. Local runtime: `Dockerfile`, `Dockerfile.dev`, `docker-compose.yml`.

Routes are thin adapters over shared services. Do not duplicate business rules across handlers. Parsers interpret files and produce typed records; they do not own HTTP or ledger transactions. The ingest service orchestrates acquisition into Parquet and ledger updates; query services read Parquet through DataFusion and apply cache/auth at the HTTP edge.

Keep COTAHIST writer and reader schemas aligned in `src/data/schema.rs` (and related ingest writers). Prefer streaming and columnar access over loading entire annual files into memory.

## Normalization, idempotency, and honesty

- Preserve official COTAHIST positional semantics: 245-byte lines, Latin-1, implied decimals, `FATCOT`, sentinel dates, headers/trailers, and `DISMES` where relevant.
- Ingestion must be idempotent by trading date (and documented natural keys). Overlapping `COTAHIST_A` / `_M` / `_D` publications must not silently duplicate rows.
- Keep financial values at the scale implied by the source layout; do not convert through lossy binary floats when exact representation matters for storage or comparison.
- COTAHIST exposes raw nominal historical prices without retroactive corporate-action adjustments. Preserve this warning in payloads, OpenAPI, and consumer documentation. Do not present derived analytics as officially published B3 series.

## REST contracts

- Use `/api/v1/...` versioned routes grouped by resource family (`cotahist`, `ibov`, ingest, health/info/metrics) consistent with `PRODUCT.md` and the published OpenAPI.
- Document every operation comprehensively in the generated OpenAPI: summary, multi-line description, parameters, closed DTOs, examples, and applicable success/error responses.
- Add Rust doc comments (`///`) to public schemas and fields so Utoipa does not publish unexplained shapes. Document authentication and the COTAHIST nominal-price warning where relevant.
- The Docusaurus site must provide task-oriented guides for consumers and operators aligned with generated OpenAPI and implemented behavior.
- Keep Docusaurus, generated OpenAPI, and implementation aligned. Link to `/openapi.json` as the machine-readable contract and distinguish it from Swagger UI at `/docs`. Swagger UI loads the same OpenAPI document; requests remain subject to each route's authentication and authorization.
- Public contracts must be closed and typed. Do not expose internal structures as substitutes for domain DTOs.
- Apply Bearer/API-key authentication to protected routes and keep public exceptions (health, docs, OpenAPI) explicit and tested. Do not extend those exceptions to ingestion operations.
- List endpoints require documented filters, pagination defaults/limits, and deterministic ordering.
- Serialize dates in ISO format; keep response envelopes consistent (`data`/`meta` or `error`).

## Testing rules

- Cover observable invariants and regressions, including failure and recovery, without mirroring the implementation in assertions.
- Test COTAHIST parsers against byte width, encoding, headers/trailers, implied decimals, sentinel dates, `FATCOT`, `DISMES`, and isolation of invalid lines. Cover ZIP archives with multiple members.
- Prove idempotent ingestion for repeated upload/folder ingest and overlapping date coverage.
- Mock external HTTP only when the code under test is not the local server; live acquisition checks must be explicit and separate.
- Public changes require REST and OpenAPI tests for authentication, schemas, filters, pagination, and the COTAHIST warning where applicable.

## Required documentation

When a change affects endpoints, payloads, filters, fields, configuration, or operational flows, update the corresponding artifacts in the same delivery:

- Utoipa-generated OpenAPI, including detailed descriptions of fields, units, sources, parameters, examples, errors, authentication, and limitations.
- Docusaurus pages under `docusaurus/docs/`, with consumer and operator guides that match the generated OpenAPI and implemented behavior.
- README and consumer/operational documentation in `docs/`, reflecting behavior actually available.
- `docs/frontend_api_changelog.md` and/or `docs/consumers_changelog.md` for consumer-visible changes; record session date, endpoints, fields, and compatibility impact.
- `.env.example`, Compose, and runtime instructions when environment variables or operational dependencies change.
- `PRODUCT.md`, context documents, and ADRs when a decision changes purpose, scope, vocabulary, or target architecture. Do not weaken product principles to justify an incomplete implementation.

Do not document planned capabilities as delivered. New pages must clearly distinguish public contracts, operations, and architectural decisions.
- Any public contract change must update the matching OpenAPI operation/schema and Docusaurus endpoint/schema pages in the same delivery. Run the Docusaurus production build after edits and report its result.

## Delivery checklist

1. Confirm alignment with `PRODUCT.md`, domain boundaries, and relevant architectural decisions.
2. Preserve provenance, ingestion idempotency, nominal-price honesty, and compatibility of affected contracts.
3. Add regression tests when applicable.
4. Run formatting, Clippy, and tests; confirm required integration paths actually ran.
5. Update OpenAPI, consumer/operational documentation, and the changelog when applicable.
6. Build Docusaurus when affected, then validate builds and execution of affected components; report results and remaining gaps without confusing intent with implementation.
7. Commit the changes by task or feature when asked. Use conventional commits.
