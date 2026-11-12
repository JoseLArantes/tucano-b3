# Frontend API changelog

## 2026-11-12

### Added

- `POST /api/v1/ingest/bucket`
  - Triggers ingestion of COTAHIST `.ZIP` files stored in an S3-compatible bucket (Cloudflare R2).
  - Optional body: `{ "prefix": "cotahist/" }`. Omit the body to use `TUCANO_S3_PREFIX`.
  - `400` when object storage is not configured or no COTAHIST ZIP is found.
  - Success body matches the other ingest routes (`task_id`, `files_processed`, `total_records_parsed`, `total_records_new`, `total_records_skipped`, `success`, `message`).
  - Compatibility: additive. OpenAPI title is `API dados de mercado da B3`.

- COTAHIST analytics (additive; all use nominal unadjusted prices and include `aviso`):
  - `GET /api/v1/cotahist/tickers/{ticker}/returns` — period return / annualized return (`desde` required).
  - `GET /api/v1/cotahist/tickers/{ticker}/stats` — market-card aggregates (extremes, vol, ADV, ATR proxy).
  - `GET /api/v1/cotahist/tickers/{ticker}/drawdown` — max and current drawdown on `pre_ult`.
  - `GET /api/v1/cotahist/tickers/{ticker}/range-position` — close position in period high–low range.
  - `GET /api/v1/cotahist/tickers/{ticker}/relative` — excess return vs `benchmark=IBOV` on aligned dates.
  - `GET /api/v1/cotahist/dates/{date}/movers` — gainers / losers / volume / trades leaders.
  - `GET /api/v1/cotahist/liquidity` — cross-sectional ADV / trades screen (paginated).
  - `GET /api/v1/cotahist/tickers/{ticker}/gaps` — open vs prior close gaps.
  - `GET /api/v1/cotahist/tickers/{ticker}/rolling` — SMA / EMA / rolling vol / ADV (`janela` 2–252).
  - `GET /api/v1/cotahist/tickers/{ticker}/spread` — bid/ask spread from COTAHIST offer fields.
  - `GET /api/v1/cotahist/compare` — multi-ticker base-100 comparison (`tickers=VALE3,PETR4`).
  - Compatibility: additive; does not change existing quote/series contracts. Derived fields are not official B3 series.

- `GET /api/v1/cotahist/tickers/{ticker}/series`
  - Compact historical series for charts: `date_ms`, `pre_ult`, `vol_tot`, `tot_neg`.
  - Required query param: `desde` (YYYY-MM-DD). Optional: `ate`.
  - Ordered ascending by date. No pagination.
  - Compatibility: additive; does not change existing `/quotes` contract.

- `GET /api/v1/cotahist/tickers/{ticker}/pre-ult/min`
  - Minimum closing price (`pre_ult`) in a period, with the session date of the extreme.
  - Required: `desde`. Optional: `ate`.
  - Compatibility: additive.

- `GET /api/v1/cotahist/tickers/{ticker}/pre-ult/max`
  - Maximum closing price (`pre_ult`) in a period, with the session date of the extreme.
  - Required: `desde`. Optional: `ate`.
  - Compatibility: additive.
