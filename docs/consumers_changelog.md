# Consumers changelog

## 2026-11-12

### Added

- `POST /api/v1/ingest/bucket`
  - Ingests COTAHIST `.ZIP` objects from S3-compatible storage (Cloudflare R2).
  - Configuration is environment-only for credentials: `TUCANO_S3_ENDPOINT`, `TUCANO_S3_BUCKET`, `TUCANO_S3_REGION` (`auto` for R2), `TUCANO_S3_ACCESS_KEY_ID`, `TUCANO_S3_SECRET_ACCESS_KEY`, optional `TUCANO_S3_PREFIX`, `TUCANO_S3_FORCE_PATH_STYLE` (default true).
  - Does not run on a schedule. Optional JSON body `{"prefix":"cotahist/"}` overrides the configured prefix for that call.
  - Same idempotent trading-date ledger and 500 MB ZIP limit as upload. Response envelope is unchanged (`IngestResponse`).
  - Compatibility: additive. OpenAPI `info.title` is now `API dados de mercado da B3`.

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
