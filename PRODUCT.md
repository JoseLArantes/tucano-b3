# Product

## Register

`tucano-b3` (Tucano API) — a standalone high-performance REST service that ingests official public B3 market-data files and serves them through typed, documented HTTP APIs.

## Users

Professional financial analysts, quantitative researchers, auditors, backoffice operators, compliance teams, and fintech builders in Brazil who need reliable access to historical quotations, ticker identity, IBOVESPA series, and derived analytics without parsing legacy B3 files themselves.

## Product Purpose

Provide a lucid, precise, and efficient Rust service that:

1. **Ingests** official public B3 archives (primarily COTAHIST ZIP/TXT and IBOV historical series) into columnar storage with idempotent, auditable processing.
2. **Exposes** that data through a versioned REST API (`/api/v1/...`) with closed, typed contracts, OpenAPI documentation, authentication, rate limiting, caching, and observability.
3. **Preserves** official source semantics — including nominal unadjusted historical prices — so consumers can trust provenance and know the limits of what they are reading.

The product is a single deployable binary with no mandatory external database server at runtime: processed data lives in year-partitioned Parquet files; an embedded SQLite ledger tracks ingestion identity and ticker registry.

## Scope

### In scope

- COTAHIST ingestion from ZIP or folder, with streaming parse, Latin-1 handling, implied decimals / `FATCOT`, and idempotent trading-date ledger control.
- COTAHIST REST: ticker discovery, quote history, latest, date ranges, compact series, extremes, and additive analytics derived from nominal prices.
- IBOV REST: historical series, latest point, and date-range queries.
- In-memory query cache (TTL + LRU), Bearer/API-key auth, per-key and per-IP rate limiting.
- OpenAPI 3 generated from code, Swagger UI at `/docs`, health/info/metrics endpoints.
- Structured logging, request IDs, Prometheus metrics, graceful shutdown, Docker packaging.

### Out of scope

- WebSocket streaming, GraphQL, OAuth2/JWT user accounts, multi-tenancy, or an operator dashboard.
- Paid or non-public B3 feeds; corporate-action–adjusted official series presented as B3 originals.
- Distributed cache (Redis) or an external relational database as a serving dependency.
- Treating derived analytics as officially published B3 series.

## Brand Personality

- Lucid
- Restrained
- Precise
- Well documented
- Well tested
- Scalable
- Secure
- Maintainable
- Performant

## Design Principles

- **Official sources only.** Persist and serve public B3 file families with explicit provenance; never invent market values.
- **Closed, typed contracts.** Prefer OpenAPI-first, source-specific DTOs over opaque payloads; version under `/api/v1/`.
- **Financial precision.** Keep decimal scales, quotation factors, currencies, and units explicit; do not launder money through binary floats.
- **Idempotent ingestion.** Overlapping annual/monthly/daily COTAHIST publications must not duplicate or corrupt series; retries must be safe.
- **Nominal-price honesty.** COTAHIST values are raw historical prices without retroactive corporate-action or inflation adjustment. Surface that warning in payloads, OpenAPI, and consumer docs.
- **Derived ≠ official.** Analytics (returns, drawdown, relative vs IBOV, movers, rolling windows, etc.) are convenience layers on nominal data and must be labeled as such.
- **Performance without ceremony.** Favor in-process columnar queries, hot-path caching, and a lean single binary over heavyweight runtime topology.
- **Security as default.** Protect mutating and data routes with explicit auth; keep public exceptions (health/docs) narrow and tested.
- **Observable operations.** Structured logs, stable request IDs, and Prometheus metrics are part of the product, not afterthoughts.
- **Document implemented behavior.** Consumer and operator docs describe what ships; do not present roadmap ideas as available capability.
- **Language policy.** User-facing API messages and public product docs in Brazilian Portuguese; code identifiers, agent docs (`AGENTS.md`, this file), and internal logs in English. Preserve official B3 field and file names.

## Non-negotiable invariants

- Ingestion and serving schemas for a given file family stay aligned inside this service.
- A trading date already accepted by the ledger is not rewritten casually; reprocessing follows an explicit, intentional path.
- List endpoints document filters, pagination defaults/limits, and deterministic ordering.
- Responses use a consistent success/error envelope; error messages remain understandable in Brazilian Portuguese.
- Performance figures published as results must be measured; targets in guides are not evidence until reproduced.

## Inclusion

- Tests
- Documentation
- Explicit types
- Type checking
- Linting
- Coverage
- Performance
- Security
- Data provenance
- Auditability of ingestion

## Sources of truth

| Concern | Anchor |
| --- | --- |
| Product purpose, scope, and principles | This file (`PRODUCT.md`) |
| Implemented API and payload contracts | Generated OpenAPI (`/openapi.json`), code, and tests |
| Consumer and operator guides | Docusaurus site under `docusaurus/` |
| Domain vocabulary | `CONTEXT-MAP.md` and related `CONTEXT.md` files |
| Agent working rules and validation | `AGENTS.md` |
| Architectural trade-offs | `docs/adr/` when present |

When purpose or scope conflicts with implementation convenience, change the code or explicitly revise this document — do not silently redefine the product.
