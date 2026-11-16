---
title: Ações e units verificadas
sidebar_position: 1
---

# Endpoints de ações e units

As rotas ficam sob `/api/v1/cotahist/equities`. Elas leem o conjunto versionado `equities-v1`, que é reconstruído a partir dos arquivos COTAHIST oficiais e separado do ledger legado de ingestão.

## Autenticação

Envie o token configurado no serviço em `Authorization: Bearer <token>` ou `X-API-Key: <token>`. A lista, o detalhe e as cotações exigem autenticação.

## Pesquisar listagens

`GET /api/v1/cotahist/equities`

Parâmetros:

| Nome | Tipo | Padrão | Limite e comportamento |
|---|---|---|---|
| `busca` | texto | vazio | Prefixo de ticker, sem diferenciar maiúsculas de minúsculas; só letras e números. |
| `pagina` | inteiro | `1` | Paginação começando em 1. |
| `por_pagina` | inteiro | `50` | Limitado a 200. |
| `ativo` | booleano | `true` | Quando verdadeiro, mostra apenas o par ticker/ISIN atual do ledger local. |

A qualificação é aplicada antes da paginação: mercado à vista `010`, BDI `02`, moeda BRL, ISIN válido e especificação ESPECI de ação ordinária (`ON`), preferencial (`PN`) ou unit (`UNT`). Códigos BDI e de mercado originais são preservados. ETFs, FIIs e outros fundos ficam fora desta rota.

A resposta inclui `data.equities`, `data.paginacao` e `data.coverage`. Cobertura indica os pregões encontrados no dataset verificado, mesmo quando `busca` limita os resultados.

```bash
curl -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  "https://b3-tucano.beakcloud.com/api/v1/cotahist/equities?busca=PETR&pagina=1&por_pagina=20"
```

## Consultar uma listagem ativa

`GET /api/v1/cotahist/equities/{ticker}` retorna o par ticker/ISIN exato, especificação de segurança, códigos brutos, período observado e indicador de atividade. Um ticker inativo, ETF ou instrumento sem ISIN elegível retorna `404`.

Use o par ticker/ISIN devolvido para uma seleção. Ticker pode ser reutilizado ao longo do tempo; o ISIN é necessário para distinguir identidades.

## Consultar o último fechamento por ISIN

`GET /api/v1/cotahist/equities/by-isin/{isin}/latest` retorna a observação mais recente disponível dentro da cobertura verificada. Sem observação, retorna `404` (`NO_DATA_IN_RANGE`).

## Consultar histórico por ISIN

`GET /api/v1/cotahist/equities/by-isin/{isin}/quotes?desde=AAAA-MM-DD&ate=AAAA-MM-DD`

Ambas as datas são obrigatórias, inclusivas, válidas em ISO 8601 e limitadas a no máximo 366 dias de diferença. Uma consulta sem pontos retorna uma lista vazia com status HTTP `200`.

Cada fechamento tem `close_per_share` como decimal exato, a data do pregão, ticker, ISIN, fator de cotação, códigos BDI e TPMERC e horário da consulta. O valor usa `PREULT` com duas casas implícitas dividido pelo `FATCOT` da linha. Preços são nominais, sem ajuste retroativo por eventos corporativos.

```bash
curl -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  "https://b3-tucano.beakcloud.com/api/v1/cotahist/equities/by-isin/BRPETRACNOR9/quotes?desde=2026-10-01&ate=2026-10-31"
```

Os campos e envelopes estão descritos em [schemas de ações e units](../schemas/equities.md). Para os arquivos de origem e o backfill, consulte [cobertura verificada](../operations/equity-data.md).
