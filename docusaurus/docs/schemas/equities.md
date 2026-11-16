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

Consulte `/openapi.json` para os schemas fechados publicados pelo serviço e as respostas de erro.
