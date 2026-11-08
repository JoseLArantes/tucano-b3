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

`identity` contém `instrument_id` (`UUID` ou `null`) e `status`: `resolved`, `unresolved` ou `ambiguous`. Resolver uma associação não altera o preço nem sua proveniência. `provenance` é obrigatória em cada ocorrência e permite localizar o fato de volta à fonte preservada:

| Campo | Tipo | Interpretação |
| --- | --- | --- |
| `source_file_id` | string UUID | Arquivo fonte preservado. |
| `source_file_type` | string | Família COTAHIST, distinguindo arquivo diário, mensal ou anual. |
| `sha256` | string hexadecimal | Digest SHA-256 dos bytes do artefato bruto. |
| `source_member_id` | string UUID | Member inventariado dentro do arquivo. |
| `source_member_name` | string | Nome/caminho do member oficial de origem. |
| `record_index` | inteiro | Índice do registro no member, iniciado em 1. |

`internal` é omitido quando `include_internal` está ausente ou falso. Com `include_internal=true`, o objeto contém `version_id` (UUID da versão de negócio selecionada) e `business_hash` (hash canônico do conteúdo de negócio). Esses valores não substituem `provenance`, não mudam a revisão e não concedem autorização adicional.

## Cobertura

QuotePage contém disclaimer e coverage. Coverage tem requested_from, requested_to, status, first_observed, last_observed, rejected, unresolved e calendar_status. status pode ser complete, partial, unknown ou not_applicable. first_observed e last_observed não provam sequência completa de pregões.

## Limitação semântica

Todos os preços são nominais sem ajuste retroativo. PREULT não deve ser transformado em campo de fechamento oficial genérico; traded_volume não deve ser confundido com traded_quantity.

## Eventos da exportação NDJSON

`GET /api/v1/b3/cotahist/quotes/export` envia uma linha JSON UTF-8 por evento. As linhas são independentes e terminam com LF. O schema `QuoteExportEvent` no `/openapi.json` define as formas fechadas; use a URL de OpenAPI da API do ambiente em uso.

| `type` | Campos | Interpretação |
| --- | --- | --- |
| `meta` | `schema_version`, `ticker_symbol`, `data_revision`, `resumed_after`, `disclaimer` | Identifica a versão do protocolo, ticker normalizado e revisão imutável antes das cotações. `resumed_after` é `null` na primeira chamada ou repete o cursor da chamada retomada. `disclaimer` preserva o aviso de preços nominais sem ajustes. |
| `quote` | `data: HistoricalQuote` | Ocorrência oficial; conserva decimais como strings, data ISO, identidade temporal e proveniência até arquivo/member/registro. |
| `checkpoint` | `cursor`, `records_sent` | Cursor exclusivo depois de uma página interna de até 500 fatos. `records_sent` acumula as linhas `quote` desta resposta até o checkpoint e reinicia em zero na retomada. |
| `complete` | `record_count`, `last_cursor`, `content_sha256` | Confirma o fim desta resposta. `record_count` conta `quote` nesta resposta. `content_sha256` cobre os bytes UTF-8, em ordem, de todas as linhas anteriores, incluindo LF final e sem a linha `complete`. |
| `error` | `code`, `message`, `request_id` | Falha segura depois que os headers HTTP já foram enviados. A implementação usa `EXPORT_FAILED`; a resposta fica incompleta e não pode ser armazenada como exportação válida. |

O cursor é Base64 URL-safe sem padding ligado à versão do schema, ticker normalizado, revisão e última chave `(trade_date, id)`. Trate-o como valor opaco do protocolo: preserve bytes/caracteres exatamente e envie junto ao mesmo ticker. Não é Bearer nem credencial, não deve ser editado ou construído pelo cliente. `data_revision` pode ser omitida na retomada, pois o cursor a conserva; se for enviada, deve coincidir. O cursor é exclusivo: a retomada começa depois do último fato já confirmado.

A resposta retomada gera seu próprio `meta`, checkpoints, contagem e checksum. `record_count` conta somente os registros desta chamada; some as contagens de segmentos concluídos para o total acumulado. Um segmento retomado que já não tenha fatos depois do cursor pode concluir com zero `quote`. Sem `complete`, não há confirmação de integridade, mesmo que o corpo termine sem erro aparente.

`include_internal=true` acrescenta `internal.version_id` e `internal.business_hash` dentro de cada `HistoricalQuote`; por padrão, a propriedade é omitida em vez de serializada como `null`. Esses valores não substituem `provenance` e não mudam a revisão. O ETag HTTP representa todos os bytes da representação pronta selecionada (gzip ou NDJSON sem compressão), enquanto `content_sha256` do evento final representa somente as linhas NDJSON anteriores a `complete`. Portanto, os hashes têm escopos diferentes e não são intercambiáveis.

Para verificar a integridade, calcule SHA-256 incrementalmente sobre os bytes UTF-8 recebidos de cada linha antes de `complete`, incluindo o LF. Não faça parse e serialização do JSON antes de calcular, pois qualquer mudança em espaçamento ou ordem de chaves mudaria o digest. Persista um checkpoint apenas depois de persistir todos os `quote` anteriores a ele.
