# Tucano B3 - Inventory of Effective Contracts and Operational Behavior (R1)

Generated as part of the review plan defined in `docs/19-plan.md` (§4 R1).
Baseline commit: `1e7ec9de97466376e85f37cdfad6a0615ed80641`

## 1. Authentication and Authorization Model

- **Protected Path Prefix**: `/api/v1/b3/*` enforced via `axum::middleware::from_fn_with_state(app_state, require_bearer_token)`.
- **Public Exceptions**:
  - `/health`
  - `/health/worker`
  - `/metrics`
  - `/openapi.json`
  - `/docs` (Swagger UI)
  - `/redoc` (Redoc UI)
- **Token Configuration**: `B3_API_BEARER_TOKEN` (default: `dev-token`). Missing or invalid header returns `401 Unauthorized` with JSON payload `{"error":"Unauthorized","message":"Token de autenticação ausente ou inválido. Use 'Authorization: Bearer <token>'."}`.

---

## 2. Effective Endpoints Matrix (42 Endpoints)

| Group / Tag | Method | Path | Summary & Business Scope | Invariants & Filters |
|---|---|---|---|---|
| **Health & Metrics** | `GET` | `/health` | API & Database connectivity | Public |
| | `GET` | `/health/worker` | Background worker status | Public |
| | `GET` | `/metrics` | Prometheus metrics scrape | Public |
| **BVBG.028.02 (Instruments)** | `GET` | `/api/v1/b3/instruments` | Instruments root list | `limit` (1-1000), `offset`, `ticker`, `isin`, `security_id`, `market_code` |
| | `GET` | `/api/v1/b3/instruments/{id}` | Instrument by primary UUID | 404 on missing |
| | `GET` | `/api/v1/b3/instruments/by-ticker/{ticker_symbol}` | Instrument by ticker | Window-resolved |
| | `GET` | `/api/v1/b3/instruments/by-isin/{isin}` | Instrument by ISIN code | Window-resolved |
| | `GET` | `/api/v1/b3/instruments/{id}/identifiers` | Temporal identifier history | `valid_from`, `valid_to` history |
| **Tradable Security List** | `GET` | `/api/v1/b3/tradable-securities` | Tradable securities list | `reference_date`, `ticker`, `market_code` |
| | `GET` | `/api/v1/b3/tradable-securities/by-date/{reference_date}` | Snapshot by market date | Daily session slice |
| | `GET` | `/api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}` | Series by ticker symbol | Chronological order |
| **BVBG.086.01 (PriceReport)** | `GET` | `/api/v1/b3/price-report/quotes` | Complete market price report | `trade_date`, `ticker`, `security_id` |
| | `GET` | `/api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}` | PriceReport quotes by ticker | Historical quotes |
| | `GET` | `/api/v1/b3/price-report/quotes/by-date/{trade_date}` | Full session quotes | Session slice |
| | `GET` | `/api/v1/b3/price-report/quotes/{id}` | PriceReport quote by UUID | Detailed quote record |
| **BVBG.186.01 (Equities Simplified)** | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes` | Equities simplified quotes | `trade_date`, `ticker` |
| | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}` | Equities simplified by ticker | Time series |
| | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}` | Equities simplified by date | Session slice |
| | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes/{id}` | Equities simplified by UUID | Detailed record |
| | `GET` | `/api/v1/b3/price-report-comparison/{trade_date}` | 086 vs 186 price comparison | Parity audit, `matches` boolean, `diff` |
| **BVBG.087.01 (IndexReport)** | `GET` | `/api/v1/b3/indices` | Registered official indices | Master list of indices |
| | `GET` | `/api/v1/b3/index-report/quotes` | Index settlement/close values | `trade_date`, `index_code`, `value_type` |
| | `GET` | `/api/v1/b3/index-report/quotes/by-index/{index_code}` | Index time series | By index code (e.g. `IBOV`) |
| | `GET` | `/api/v1/b3/index-report/quotes/by-date/{trade_date}` | Index values by date | Session slice |
| **Index Portfolio Composition** | `GET` | `/api/v1/b3/index-compositions` | Index portfolios | `composition_date`, `index_code` |
| | `GET` | `/api/v1/b3/index-compositions/{index_code}` | Composition by index | Theoretical portfolio |
| | `GET` | `/api/v1/b3/index-compositions/{index_code}/{composition_date}` | Specific date portfolio | Specific rebalancing snapshot |
| | `GET` | `/api/v1/b3/index-compositions/by-ticker/{ticker_symbol}` | Indices containing ticker | Reverse index membership |
| **COTAHIST Series** | `GET` | `/api/v1/b3/cotahist/quotes` | COTAHIST raw historical quotes | Unadjusted prices warning, `distribution_number` |
| | `GET` | `/api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}` | COTAHIST by ticker | Unadjusted time series |
| | `GET` | `/api/v1/b3/cotahist/quotes/by-isin/{isin}` | COTAHIST by ISIN | Unadjusted time series |
| | `GET` | `/api/v1/b3/cotahist/quotes/by-date/{trade_date}` | COTAHIST session quotes | Full market daily slice |
| | `GET` | `/api/v1/b3/cotahist/quotes/{id}` | COTAHIST by UUID | Single transaction record |
| **Company Data** | `GET` | `/api/v1/b3/company-data` | Consolidated issuer overview | `search` filter over assets/descriptions |
| **Source Files & Sync Runs** | `GET` | `/api/v1/b3/source-files` | Registered source files | `source_file_type`, `reference_date`, `status` |
| | `GET` | `/api/v1/b3/source-files/{id}` | Source file by UUID | File record & SHA-256 |
| | `GET` | `/api/v1/b3/source-files/{id}/members` | Archive member files | Extracted nested members |
| | `GET` | `/api/v1/b3/sync-runs` | Ingestion sync runs history | Audit runs list |
| | `GET` | `/api/v1/b3/sync-runs/{id}` | Sync run details by UUID | Counters: inserted, changed, unchanged |
| | `POST` | `/api/v1/b3/sync-runs` | Trigger sync execution | `ManualSyncRequest` body |
| | `POST` | `/api/v1/b3/identity/rebuild` | Trigger retroactive identity resolution | Rebuilds unresolved/ambiguous links |
| **Quality & Governance** | `GET` | `/api/v1/b3/quarantine` | Quarantined records list | Isolated malformed lines |
| | `POST` | `/api/v1/b3/quarantine/replay` | Replay quarantine records | Reprocesses valid records |
| | `GET` | `/api/v1/b3/field-changes` | Field changes audit log | `table_name`, `record_pk`, `field_name` |
| | `GET` | `/api/v1/b3/pending-updates` | Discovered pending updates | Scanner discovery queue |
| | `GET` | `/api/v1/b3/diagnostics/coverage` | Dataset coverage diagnostics | Per table min/max dates, record counts |

---

## 3. Persistent Schema Invariants

- Total tables: 15.
- Key unique constraints:
  - `b3_instruments(security_id)`
  - `b3_instrument_identifiers(identifier_type, identifier_value, COALESCE(market_code, ''), COALESCE(valid_from, '1900-01-01'))`
  - `b3_cotahist_quotes(trade_date, ticker_symbol, isin, market_type, distribution_number)`
  - `b3_price_report_quotes(trade_date, ticker_symbol, market_identifier_code, days_to_settlement)`
  - `b3_source_files(source_file_type, reference_date, sha256)`
- Field changes audit: `b3_field_changes` records `old_value_text`, `new_value_text`, `table_name`, `record_pk`, and `field_name`.

