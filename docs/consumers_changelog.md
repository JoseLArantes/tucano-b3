# Consumers changelog

## 2026-11-24

### Adicionado

- CRUD autenticado de proventos em `/api/v1/proventos` e `/api/v1/proventos/{id}`: listagem paginada com filtros por ticker, tipo e intervalo de pagamento; criação, consulta, substituição e exclusão.
- Os tipos aceitos são `Dividendo`, `JCP` e `Rendimento`. `valor` entra e sai como decimal textual em reais; a resposta usa seis casas exatas. Datas ISO e `obs` de fonte/observações podem ser nulas.
- Ledger SQLite novo recebe uma única carga inicial com 34 registros de novembro de 2026. Categorias plurais foram normalizadas para o tipo `Dividendo`; o campo `obs` mantém as observações fornecidas para os registros.
- Compatibilidade: aditivo. Sucessos usam `{ data, meta }`; proventos não são apresentados como série oficial da B3.
- A listagem aceita também `tickers`, com até 100 códigos separados por vírgula; `ticker` e `tickers` não podem ser usados juntos.
- Relatórios aditivos em `GET /api/v1/proventos/yearly`, `/period` e `/series`. A seleção usa a data-com. Os totais `proventos_on_year`, `proventos_on_period` e o `valor` dos pontos usam texto decimal com seis casas.

## 2026-11-18

### Adicionado

- `GET /api/v1/cotahist/equities/by-isin/{isin}/turnover` retorna o `VOLTOT` exato de ON, PN e units do lote padrão, com datas observadas, revisão e origem. O campo `financial_turnover_brl` é texto com duas casas. Zeros explícitos são preservados e dias sem linha são omitidos. Contrato aditivo.

- Novo recurso de calendário B3 com carga inicial de 2026 e CRUD autenticado: `GET /api/v1/calendar`, `POST /api/v1/calendar`, `GET /api/v1/calendar/{year}`, `PUT /api/v1/calendar/{year}` e `DELETE /api/v1/calendar/{year}`.
- O corpo de calendário usa `ano`, `fontes` e `eventos`. Eventos incluem data ISO, nome, situação de funcionamento e observações/horários opcionais. `POST` responde `409` para ano existente; `PUT` e `DELETE` respondem `404` para ano ausente; datas fora do ano e duplicadas são rejeitadas.
- `GET /api/v1/negotiation-hours` apresenta os horários gerais Trader publicados pela B3, no fuso `America/Sao_Paulo`: Tela 09:00–18:00, RFQ 09:00–18:00 e Voice 08:00–19:15. Horários especiais por data permanecem no calendário.
- Compatibilidade: aditivo; todas as respostas de sucesso usam `{ data, meta }`. A fonte de 2026 é preservada e o seed é aplicado somente uma vez por ledger.

## 2026-11-17

### Alterado

- Caminhos legados `GET /api/v1/ibov/history` e `GET /api/v1/ibov/latest` removidos. Consumidores devem migrar para `/api/v1/metrics/ibov/history` e `/api/v1/metrics/ibov/latest`.
- Novas séries oficiais em `/api/v1/metrics/idiv/history` e `/api/v1/metrics/ifix/latest`, alimentadas por CSVs da B3. O fechamento diário aparece em `pre_ult` como pontos do índice; campos ausentes no CSV permanecem nulos.
- Novos endpoints autenticados de ingestão manual e estado em `/api/v1/ingest/indices`, `/api/v1/ingest/indices/{id}` e `/api/v1/ingest/indices/status`.
- Compatibilidade: mudança incompatível para os dois caminhos IBOV anteriores. Schemas das respostas de índice mantêm o envelope existente.

## 2026-11-16

### Added

- Verificação e leitura de ações e units por meio do dataset versionado `equities-v1`:
  - `GET /api/v1/cotahist/equities` — pesquisa paginada com prefixo `busca`, filtro `ativo` (padrão `true`) e cobertura verificada.
  - `GET /api/v1/cotahist/equities/{ticker}` — detalhe da listagem ativa elegível.
  - `GET /api/v1/cotahist/equities/by-isin/{isin}/latest` — último fechamento disponível dentro da cobertura (`404` quando não há observação).
  - `GET /api/v1/cotahist/equities/by-isin/{isin}/quotes` — histórico com `desde` e `ate` obrigatórios e intervalo máximo de um ano (`200` com lista vazia quando não há pontos).
  - Elegibilidade exige mercado spot `010`, BDI `02`, BRL, ISIN válido e especificação `ON`, `PN` ou `UNT`, filtrados antes da paginação. BOVA11 e demais ETFs não são listados.
  - O fechamento por ação ou unit aplica o `FATCOT` ao `PREULT` com duas casas implícitas. Resposta preserva ticker, ISIN, BDI, TPMERC, data, fator, preço decimal exato e horário da consulta.
  - Compatibilidade: aditivo. As rotas legadas de tickers e cotações mantêm seus schemas e dataset COTAHIST original. Preços novos são nominais e não ajustados por eventos corporativos.

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
