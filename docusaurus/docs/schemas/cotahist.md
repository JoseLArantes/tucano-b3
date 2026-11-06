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
