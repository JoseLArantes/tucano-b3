# Tucano B3 - Inventory of Effective Contracts and Operational Behavior (R1)

Generated as part of the review plan defined in `docs/19-plan.md` (§4 R1).  
Baseline commit: `1e7ec9de97466376e85f37cdfad6a0615ed80641`

---

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

## 2. Effective Endpoints Matrix (44 Protected + 3 Public Routes)

| Group / Tag | Method | Path | Summary & Business Scope | Invariants & Filters |
|---|---|---|---|---|
| **Health & Metrics** | `GET` | `/health` | API & Database connectivity | Public |
| | `GET` | `/health/worker` | Background worker status | Public |
| | `GET` | `/metrics` | Prometheus metrics scrape | Public |
| **BVBG.028.02 (Instruments)** | `GET` | `/api/v1/b3/instruments` | Instruments root list | `limit` (1-1000), `offset`, `ticker`, `isin`, `security_id`, `market_code` |
| | `GET` | `/api/v1/b3/instruments/{id}` | Instrument by primary UUID | 404 on missing |
| | `GET` | `/api/v1/b3/instruments/by-security-id/{security_id}` | Instrument by numeric security_id | Primary root lookup |
| | `GET` | `/api/v1/b3/instruments/by-ticker/{ticker_symbol}` | Instrument by ticker | Window-resolved |
| | `GET` | `/api/v1/b3/instruments/by-isin/{isin}` | Instrument by ISIN code | Window-resolved |
| | `GET` | `/api/v1/b3/instruments/{id}/identifiers` | Temporal identifier history | `valid_from`, `valid_to` history |
| **Tradable Security List** | `GET` | `/api/v1/b3/tradable-securities` | Tradable securities list | `reference_date`, `ticker`, `market_code` |
| | `GET` | `/api/v1/b3/tradable-securities/by-date/{reference_date}` | Snapshot by market date | Daily session slice |
| | `GET` | `/api/v1/b3/tradable-securities/by-ticker/{ticker_symbol}` | Series by ticker symbol | Chronological order |
| **BVBG.086.01 (PriceReport)** | `GET` | `/api/v1/b3/price-report/quotes` | Complete market price report | `trade_date`, `ticker`, `security_id` |
| | `GET` | `/api/v1/b3/price-report/quotes/by-ticker/{ticker_symbol}` | PriceReport quotes by ticker | Historical quotes |
| | `GET` | `/api/v1/b3/price-report/quotes/by-date/{trade_date}` | Full session quotes | Session slice |
| | `GET` | `/api/v1/b3/price-report/quotes/{quote_id}` | PriceReport quote by UUID | Detailed quote record |
| **BVBG.186.01 (Equities Simplified)** | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes` | Equities simplified quotes | `trade_date`, `ticker` |
| | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes/by-ticker/{ticker_symbol}` | Equities simplified by ticker | Time series |
| | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes/by-date/{trade_date}` | Equities simplified by date | Session slice |
| | `GET` | `/api/v1/b3/equities-simplified-price-report/quotes/{quote_id}` | Equities simplified by UUID | Detailed record |
| | `GET` | `/api/v1/b3/price-report-comparison/{trade_date}` | 086 vs 186 price comparison | Parity audit, `matches` boolean, `diff` |
| **BVBG.087.01 (IndexReport)** | `GET` | `/api/v1/b3/index-report/indices` | Registered official indices | Master list of indices |
| | `GET` | `/api/v1/b3/index-report/values` | Index settlement/close values | `trade_date`, `index_code`, `value_type` |
| | `GET` | `/api/v1/b3/index-report/values/by-index/{index_code}` | Index time series | By index code (e.g. `IBOV`) |
| | `GET` | `/api/v1/b3/index-report/values/by-date/{trade_date}` | Index values by date | Session slice |
| **Index Portfolio Composition** | `GET` | `/api/v1/b3/index-composition/portfolios` | Index portfolios | `composition_date`, `index_code` |
| | `GET` | `/api/v1/b3/index-composition/portfolios/{index_code}` | Composition by index | Theoretical portfolio |
| | `GET` | `/api/v1/b3/index-composition/portfolios/{index_code}/by-date/{composition_date}` | Specific date portfolio | Specific rebalancing snapshot |
| | `GET` | `/api/v1/b3/index-composition/components/by-ticker/{ticker_symbol}` | Indices containing ticker | Reverse index membership |
| **COTAHIST Series** | `GET` | `/api/v1/b3/cotahist/quotes` | COTAHIST raw historical quotes | Unadjusted prices disclaimer, `distribution_number` |
| | `GET` | `/api/v1/b3/cotahist/quotes/by-ticker/{ticker_symbol}` | COTAHIST by ticker | Unadjusted time series |
| | `GET` | `/api/v1/b3/cotahist/quotes/by-isin/{isin}` | COTAHIST by ISIN | Unadjusted time series |
| | `GET` | `/api/v1/b3/cotahist/quotes/by-date/{trade_date}` | COTAHIST session quotes | Full market daily slice |
| | `GET` | `/api/v1/b3/cotahist/quotes/{quote_id}` | COTAHIST by UUID | Single transaction record |
| **Company Data** | `GET` | `/api/v1/b3/company-data` | Consolidated issuer overview | `search` filter over assets/descriptions |
| **Source Files & Sync Runs** | `GET` | `/api/v1/b3/source-files` | Registered source files | `source_file_type`, `reference_date`, `status` |
| | `GET` | `/api/v1/b3/source-files/{id}` | Source file by UUID | File record & SHA-256 |
| | `GET` | `/api/v1/b3/source-files/{id}/members` | Archive member files | Extracted nested members |
| | `GET` | `/api/v1/b3/sync-runs` | Ingestion sync runs history | Audit runs list |
| | `GET` | `/api/v1/b3/sync-runs/{id}` | Sync run details by UUID | Counters: inserted, changed, unchanged |
| | `POST` | `/api/v1/b3/sync-runs` | Trigger sync execution | `ManualSyncRequest` body |
| | `POST` | `/api/v1/b3/sync-runs/rebuild-identity` | Trigger retroactive identity resolution | Rebuilds unresolved/ambiguous links |
| **Quality & Governance** | `GET` | `/api/v1/b3/quarantine` | Quarantined records list | Isolated malformed lines |
| | `POST` | `/api/v1/b3/quarantine/replay` | Replay quarantine records | Reprocesses valid records |
| | `GET` | `/api/v1/b3/field-changes` | Field changes audit log | `table_name`, `record_pk`, `field_name` |
| | `GET` | `/api/v1/b3/pending-updates` | Discovered pending updates | Scanner discovery queue |
| | `GET` | `/api/v1/b3/diagnostics/coverage` | Dataset coverage diagnostics | Per table min/max dates, record counts |

---

## 3. Response Envelope and Payload Contracts

All collection endpoints return the typed `PaginatedResponse<T>` schema defined in `src/api/schemas.rs`:

```json
{
  "items": [ ... ],
  "total": 1520,
  "limit": 100,
  "offset": 0,
  "disclaimer": "AVISO: As cotacoes da serie historica COTAHIST representam precos brutos nominais de negociacao diaria, sem ajustes retroativos por dividendos, JCP, desdobramentos ou bonificacoes corporativas."
}
```

### Envelope Fields:
- `items`: `Vec<T>` — Array of domain records matching the query.
- `total`: `i64` — Total record count for the filter criteria.
- `limit`: `i64` — Maximum items clamped per page (default: 100, max: 1000).
- `offset`: `i64` — Pagination offset applied (default: 0).
- `disclaimer`: `Option<String>` — Present on raw price series (COTAHIST) reminding consumers that prices are nominal historical and unadjusted for corporate actions. Omitted when not applicable.

### Item Embedded Contracts:
Every domain item embeds provenance metadata (`provenance`):
- `source_file_type`: String (e.g. `COTAHIST_DAILY`, `BVBG_028_INSTRUMENTS_FILE`).
- `source_file_id`: String (UUID).
- `source_file_name`: Option<String> (e.g. `COTAHIST_D260301.TXT`).
- `source_member_name`: Option<String>.
- `reference_date`: Option<NaiveDate> (YYYY-MM-DD).
- `sha256`: Option<String> (SHA-256 of the raw file).
- `ingested_at`: Option<DateTime<Utc>>.

When `include_internal=true`, the item also includes `internal_metadata`:
- `data_changed_at`: Option<DateTime<Utc>> (business change timestamp).
- `field_changed_at`: Option<serde_json::Value> (granular field change timestamps).
- `resolution_status`: Option<String> (`resolved`, `unresolved`, `ambiguous`).
- `last_seen_at`: Option<DateTime<Utc>>.
- `source_line_number`: Option<i32>.

---

## 4. Persistent Schema Invariants (migrations/0001_initial_b3_schema.sql)

- Total tables in migration 0001: 15 tables.
- **Natural Key Constraints (`UNIQUE NULLS NOT DISTINCT`)**:
  - `uq_cotahist_nk`: `b3_cotahist_quotes(trade_date, ticker_symbol, market_type, bdi_code, distribution_number)`
  - `uq_price_report_nk`: `b3_price_report_quotes(trade_date, security_id, ticker_symbol, market_identifier_code, days_to_settlement)`
  - `uq_equities_simplified_nk`: `b3_equities_simplified_price_quotes(trade_date, security_id, ticker_symbol, market_identifier_code)`
  - `uq_index_report_nk`: `b3_index_report_quotes(trade_date, index_code, value_type)`
  - `uq_index_composition_nk`: `b3_index_compositions(composition_date, index_code, ticker_symbol, instrument_id)`
  - `uq_tradable_nk`: `b3_tradable_securities(reference_date, security_id, ticker_symbol, market_code)`
- **Instrument Root & Window Constraints**:
  - `b3_instruments(security_id)` — Root canonical identity.
  - `uq_instrument_identifiers`: `b3_instrument_identifiers(identifier_type, identifier_value, COALESCE(market_code, ''), COALESCE(valid_from, '1900-01-01'))`
- **Trading Calendar Table**:
  - `b3_trading_calendar(calendar_date PRIMARY KEY, is_trading_day, holiday_name, source_note, updated_at)` — already created in migration 0001.
- **Field Changes Audit Log**:
  - `b3_field_changes(table_name, record_pk, field_name, old_value_text, new_value_text, old_value_hash, new_value_hash, changed_at)`
