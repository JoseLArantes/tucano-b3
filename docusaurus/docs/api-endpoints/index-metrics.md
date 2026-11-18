---
title: Índices e ingestão CSV
---

# Séries diárias dos índices

As rotas autenticadas consultam os CSVs anuais publicados pela B3. Os valores retornados em `pre_ult` representam o fechamento diário do índice em pontos. Campos de abertura, máxima, mínima, volume e negócios ficam nulos porque não são publicados nesse arquivo. `var_pct` é calculada contra o pregão disponível anterior do mesmo índice.

| Método | Rota | Descrição |
| --- | --- | --- |
| `GET` | `/api/v1/metrics/ibov/history` | Histórico paginado do IBOV |
| `GET` | `/api/v1/metrics/ibov/latest` | Fechamento mais recente do IBOV |
| `GET` | `/api/v1/metrics/idiv/history` | Histórico paginado do IDIV |
| `GET` | `/api/v1/metrics/ifix/latest` | Fechamento mais recente do IFIX |

As rotas de histórico aceitam `desde`, `ate`, `dias`, `ordem`, `pagina` e `por_pagina`. As datas usam `AAAA-MM-DD`, a ordenação padrão é decrescente e o tamanho de página padrão é 252. Respostas seguem o envelope `{ data, meta }`. Se não houver observações para o intervalo, a API responde `404 NO_DATA_IN_RANGE`.

O caminho antigo `/api/v1/ibov/history` e `/api/v1/ibov/latest` foi removido. Use as novas rotas sob `/metrics/ibov`.

Consulte o endpoint `/openapi.json` no servidor da API para os schemas e parâmetros completos. O Swagger UI em `/docs` carrega o mesmo contrato e as chamadas seguem protegidas por autenticação.
