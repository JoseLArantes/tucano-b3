---
title: Schemas de ações e units
sidebar_position: 2
---

# Schemas de ações e units

As respostas de sucesso usam o envelope comum `data` e `meta` do serviço.

## Listagem elegível

Cada item em `data.equities` contém:

| Campo | Significado |
|---|---|
| `ticker` | Símbolo normalizado usado na pesquisa. |
| `raw_ticker` | `CODNEG` original do COTAHIST. |
| `isin` | `CODISI` oficial validado pelo dígito verificador. |
| `bdi_code` | Código BDI original. |
| `market_code` | Código TPMERC original, como `010`. |
| `company_name` | Nome de emissor presente no arquivo, quando houver. |
| `security_specification` | Campo ESPECI, como `ON`, `PN` ou `UNT`. |
| `first_seen`, `last_seen` | Primeiro e último pregões observados para aquele par ticker/ISIN na cobertura disponível. |
| `is_active` | Indica se o ledger local aponta esse mesmo par como atual. |

`data.coverage` contém `from` e `to`, as primeiras e últimas datas cobertas para os registros de ações e units verificados.

## Observação de fechamento

Cada item de `data` no histórico, ou o conteúdo de `data` em `/latest`, contém:

| Campo | Significado |
|---|---|
| `ticker`, `isin` | Identidade observada no arquivo. |
| `bdi_code`, `market_code` | Códigos brutos preservados do arquivo. |
| `data` | Data do pregão em `AAAA-MM-DD`. |
| `close_per_share` | Decimal textual exato por ação ou unit após aplicar o fator de cotação. |
| `quotation_factor` | `FATCOT` oficial, inteiro positivo. |
| `source` | Identifica a origem COTAHIST spot. |
| `retrieved_at` | Horário UTC da consulta. |
| `availability` | `available` para observações retornadas. |
| `adjusted_for_corporate_actions` | Sempre `false`; o histórico nominal não recebe ajustes retroativos. |

## Paginação e precisão

`data.paginacao` contém total, página e tamanho de página. A paginação é determinística por ticker e ISIN. `close_per_share` é texto decimal para evitar arredondamento binário na conversão do preço oficial. Pontos fora da cobertura verificada não são estimados nem preenchidos.

## Observação de volume financeiro

`data.observations` contém os registros disponíveis. `data.observation_count` conta esses registros e `data.dataset_revision` identifica o conjunto ativo ou vale `null` antes da primeira publicação.

| Campo | Significado |
|---|---|
| `data`, `isin`, `ticker` | Data do pregão, identidade verificada e ticker oficial naquele pregão. |
| `currency`, `segment` | `BRL` e `standard_lot` no piloto. |
| `financial_turnover_brl` | Valor em reais como texto decimal com duas casas. |
| `voltot_raw` | 18 dígitos originais de `VOLTOT`, incluindo zeros à esquerda. |
| `source_member`, `source_line` | Nome do TXT de origem e linha física, começando em 1. |

Exemplos sintéticos para ilustrar a leitura. Os valores não foram publicados pela B3.

| Data | Classe | ISIN | Ticker | `voltot_raw` | `financial_turnover_brl` |
|---|---|---|---|---|---|
| 2026-01-05 | ON | BRPETRACNOR9 | PETR3 | `009007199254740993` | `90071992547409.93` |
| 2026-01-05 | PN | BRPETRACNPR6 | PETR4 | `000000000001000001` | `10000.01` |
| 2026-01-05 | Unit | BRSANBCDAM13 | SANB11 | `000000000000020000` | `200.00` |
| 2026-01-06 | ON | BRPETRACNOR9 | PETR3 | `000000000000000000` | `0.00` |

Nos exemplos, `FATCOT=1000` afeta a cotação por ação ou unit. O volume resulta diretamente das duas casas implícitas de `VOLTOT`. A ausência de uma observação ON em 07/01 exclui essa data da série do exemplo.

Consulte `/api/docs/openapi.json` para os schemas e as respostas de erro.
