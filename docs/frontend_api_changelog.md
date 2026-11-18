# Frontend API changelog

## 2026-11-18

### Adicionado

- Calendário anual B3 em `/api/v1/calendar`: `GET` lista anos, `POST` cria, `GET /{year}` consulta, `PUT /{year}` substitui por inteiro e `DELETE /{year}` remove. O ano de 2026 é semeado uma única vez no ledger SQLite com fontes B3; alterações são persistentes. Escritas precisam de credencial configurada e usam autenticação.
- Eventos expõem `data`, `evento`, `situacao` (`closed`, `special_hours`, `open`), `descricao` e horários opcionais. Datas precisam corresponder ao ano, não podem se repetir e são retornadas em ordem crescente. O seed inclui o funcionamento especial de 18/02/2026 e funcionamento normal em 09/07/2026.
- `GET /api/v1/negotiation-hours` retorna horários gerais da página Trader da B3: Tela e RFQ 09:00–18:00; Voice 08:00–19:15; fuso `America/Sao_Paulo`.
- Compatibilidade: aditivo; sucesso mantém envelope `{ data, meta }`. Consulte `/openapi.json` e as páginas de calendário no Docusaurus.

## 2026-11-17

### Alterado

- Rotas IBOV antigas removidas. Atualize `GET /api/v1/ibov/history` para `GET /api/v1/metrics/ibov/history` e `GET /api/v1/ibov/latest` para `GET /api/v1/metrics/ibov/latest`.
- Incluídas `GET /api/v1/metrics/idiv/history` e `GET /api/v1/metrics/ifix/latest`. O CSV oficial só contém níveis de fechamento, então demais campos OHLC e volume ficam nulos.
- Para operações, `POST /api/v1/ingest/indices` inicia a execução manual; `GET /api/v1/ingest/indices/{id}` consulta o trabalho e `/api/v1/ingest/indices/status` informa a última ingestão bem-sucedida e a última data de pregão carregada.
- Compatibilidade: alteração incompatível dos caminhos IBOV antigos. As novas operações de índices usam autenticação e o envelope de resposta vigente.

## 2026-11-16

### Added

- Rotas autenticadas para picker e cotações verificadas de ações ON/PN e units:
  - `GET /api/v1/cotahist/equities` com busca por prefixo, paginação, filtro de listagem ativa e intervalo de cobertura.
  - `GET /api/v1/cotahist/equities/{ticker}` para consultar uma listagem atual por ticker.
  - `GET /api/v1/cotahist/equities/by-isin/{isin}/latest` para o último fechamento nominal disponível.
  - `GET /api/v1/cotahist/equities/by-isin/{isin}/quotes` para o histórico com intervalo obrigatório de até um ano.
  - A listagem mostra ticker e ISIN juntos para resolver a identidade; inclui apenas mercado `010`, BDI `02`, BRL, ISIN válido e classe `ON`, `PN` ou `UNT`. Unidades de ETF/FII são excluídas.
  - Preços usam `PREULT / 100 / FATCOT`, são decimais exatos e não recebem ajustes retroativos por eventos corporativos.
  - Compatibilidade: aditivo; rotas legadas de ticker e quote não mudaram.

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
