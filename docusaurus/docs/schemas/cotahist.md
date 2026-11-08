---
title: Schema COTAHIST
---

# Schema de cotações COTAHIST

HistoricalQuote representa uma ocorrência COTAHIST e inclui id, trade_date, identity, provenance e internal opcional. Ticker pode ser nulo ou não resolver para um instrumento.

## Campos de negócio

| Campo | Tipo | Interpretação |
| --- | --- | --- |
| ticker_symbol | string/null | Código publicado na ocorrência |
| market_type, bdi_code | string/null | Qualificadores originais de mercado |
| distribution_number | string/null | DISMES; parte da identidade granular do fato |
| term_days | string/null | Prazo publicado para operação a termo |
| issuer_short_name, paper_specification | string/null | Textos publicados |
| currency | string/null | Moeda original, sem conversão |
| open_price, high_price, low_price, average_price | decimal string/null | Preços nominais publicados |
| last_price | decimal string/null | PREULT; preço do último negócio |
| best_bid_price, best_ask_price | decimal string/null | Melhores ofertas publicadas |
| trades_count | inteiro/null | Número de negócios, não quantidade negociada |
| traded_quantity, traded_volume | decimal string/null | Quantidade e volume com precisão textual |
| option_exercise_price | decimal string/null | Preço de exercício, quando aplicável |
| price_correction_indicator | string/null | Indicador publicado |
| maturity_date | data ISO/null | Vencimento depois de remover sentinela |
| quotation_factor | inteiro/null | FATCOT, fator de cotação |
| exercise_price_points | decimal string/null | Exercício expresso em pontos |
| isin | string/null | ISIN publicado na ocorrência |

## Proveniência, identidade e internals

identity contém instrument_id opcional e status. provenance aponta arquivo, tipo, hash, member e record_index. internal pode apresentar version_id e business_hash quando include_internal=true.

## Cobertura

QuotePage contém disclaimer e coverage. Coverage tem requested_from, requested_to, status, first_observed, last_observed, rejected, unresolved e calendar_status. status pode ser complete, partial, unknown ou not_applicable. first_observed e last_observed não provam sequência completa de pregões.

## Limitação semântica

Todos os preços são nominais sem ajuste retroativo. PREULT não deve ser transformado em campo de fechamento oficial genérico; traded_volume não deve ser confundido com traded_quantity.

## Eventos da exportação NDJSON

`GET /api/v1/b3/cotahist/quotes/export` envia uma linha JSON UTF-8 por evento. As linhas são independentes e terminam com LF. O schema `QuoteExportEvent` no `/openapi.json` define as formas fechadas; use a URL de OpenAPI da API do ambiente em uso.

| `type` | Campos | Interpretação |
| --- | --- | --- |
| `meta` | `schema_version`, `ticker_symbol`, `data_revision`, `resumed_after`, `disclaimer` | Identifica a projeção e revisão imutável antes das cotações. `disclaimer` preserva o aviso de preços nominais sem ajustes. |
| `quote` | `data: HistoricalQuote` | Ocorrência oficial; conserva decimais como strings, data ISO, identidade temporal e proveniência até arquivo/member/registro. |
| `checkpoint` | `cursor`, `records_sent` | Cursor opaco depois do último fato de um lote. `records_sent` conta as linhas `quote` desta resposta, não o total histórico. |
| `complete` | `record_count`, `last_cursor`, `content_sha256` | Confirma o fim da resposta. `content_sha256` cobre, em ordem, todas as linhas anteriores, incluindo `meta`, cotações e checkpoints, sem a linha `complete`. |
| `error` | `code`, `message`, `request_id` | Falha segura depois que os headers HTTP já foram enviados. A resposta fica incompleta e não pode ser armazenada como exportação válida. |

O cursor é Base64 URL-safe sem padding de um token opaco ligado à versão do schema, ticker, revisão e chave `(trade_date, id)`. Ele é exclusivo: a retomada começa depois do último fato já confirmado. Preserve o cursor exatamente e envie novamente o mesmo ticker. A resposta retomada gera seu próprio `meta`, checkpoints, contagem e checksum; some as contagens se precisar do total entre segmentos. Sem `complete`, não há confirmação de integridade, mesmo que o corpo termine sem erro aparente.

`include_internal=true` acrescenta `internal.version_id` e `internal.business_hash` dentro de cada `HistoricalQuote`. Esses valores não substituem `provenance` e não mudam a revisão. O ETag HTTP representa os bytes da representação pronta (gzip ou NDJSON sem compressão), enquanto `content_sha256` do evento final representa as linhas NDJSON anteriores a `complete`.
