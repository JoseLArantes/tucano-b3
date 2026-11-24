---
title: CRUD de proventos
---

# CRUD de proventos

O recurso mantém registros de `Dividendo`, `JCP` e `Rendimento` no ledger SQLite. Cada lançamento inclui ticker, valor por ação ou cota, datas disponíveis e `obs` para fonte e observações. A API preserva valores em escala decimal exata de seis casas e não interpreta estes registros como uma série oficial da B3. A proveniência informada em `obs` é armazenada literalmente; a API não consulta nem valida a fonte.

As rotas exigem Bearer ou API Key quando a autenticação está habilitada. `POST`, `PUT` e `DELETE` retornam `503` se não houver credencial configurada. Sucessos usam `{ data, meta }`; erros usam `{ error }`.

## Operações

| Método | Rota | Uso |
| --- | --- | --- |
| `GET` | `/api/v1/proventos` | Lista registros com filtros e paginação. |
| `POST` | `/api/v1/proventos` | Cadastra um provento (`201`). |
| `GET` | `/api/v1/proventos/{id}` | Consulta um registro por identificador. |
| `PUT` | `/api/v1/proventos/{id}` | Substitui os campos de negócio do registro. |
| `DELETE` | `/api/v1/proventos/{id}` | Exclui o registro persistido. |

## Consulta e filtros

`GET /api/v1/proventos` aceita os parâmetros opcionais `ticker`, `tipo`, `data_pagamento_desde`, `data_pagamento_ate`, `pagina` e `por_pagina`.

| Parâmetro | Formato | Padrão/limite | Comportamento |
| --- | --- | --- | --- |
| `ticker` | texto alfanumérico | 1–10 caracteres | Comparação exata após normalização para maiúsculas. |
| `tipo` | enum | `Dividendo`, `JCP`, `Rendimento` | Filtra pela categoria do registro. |
| `data_pagamento_desde` | `AAAA-MM-DD` | opcional | Limite inferior inclusivo. Exclui registros sem data de pagamento. |
| `data_pagamento_ate` | `AAAA-MM-DD` | opcional | Limite superior inclusivo. Exclui registros sem data de pagamento. |
| `pagina` | inteiro | `1`, mínimo `1` | Página baseada em 1. Valores menores são normalizados para 1. |
| `por_pagina` | inteiro | `50`, mínimo `1`, máximo `1000` | A API limita valores fora do intervalo permitido. |

Quando os dois limites de pagamento são informados, `desde` deve anteceder ou ser igual a `ate`. A ordem dos resultados é data de pagamento crescente, ticker, tipo e ID; registros sem data de pagamento ficam no final. A resposta inclui `data.proventos` e `data.paginacao` com `total`, `pagina`, `por_pagina` e `total_paginas`.

```bash
curl -s -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  'http://localhost:8080/api/v1/proventos?ticker=PETR4&tipo=JCP&data_pagamento_desde=2026-11-01&data_pagamento_ate=2026-11-30' | jq
```

## Criar, substituir e excluir

Envie o corpo fechado `ProventoRequest` com `ticker`, `tipo`, `valor`, `data_com`, `data_pagamento` e `obs`. Campos desconhecidos são rejeitados. `tipo` aceita somente `Dividendo`, `JCP` ou `Rendimento`. `valor` é uma string decimal positiva em reais com até seis casas, sem sinal, separador de milhar ou arredondamento; o máximo equivale a `DECIMAL(14,6)`. A API responde sempre com seis casas. `data_com` e `data_pagamento` aceitam `null`; quando ambas existem, a data-com não pode ser posterior ao pagamento. Datas inválidas ou fora de `AAAA-MM-DD` são rejeitadas. `ticker` é normalizado para maiúsculas e aceita de 1 a 10 caracteres alfanuméricos. `obs` aceita até 255 caracteres e deve registrar a origem quando conhecida.

Cada `POST` cria um novo ID, mesmo se outro provento tiver os mesmos campos. `PUT` substitui todos os campos de negócio e mantém o ID e `criado_em`. `DELETE` remove permanentemente o registro. Não há deduplicação automática.

```bash
curl -X POST http://localhost:8080/api/v1/proventos \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "ticker": "PETR4",
    "tipo": "JCP",
    "valor": "0.674100",
    "data_com": "2026-08-21",
    "data_pagamento": "2026-11-23",
    "obs": "Fonte informada no cadastro; previsão de pagamento em novembro de 2026"
  }'

curl -X PUT http://localhost:8080/api/v1/proventos/35 \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"ticker":"PETR4","tipo":"JCP","valor":"0.674100","data_com":"2026-08-21","data_pagamento":"2026-11-23","obs":"Fonte revisada"}'

curl -X DELETE http://localhost:8080/api/v1/proventos/35 \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN"
```

`POST` responde `201`; `PUT` e `DELETE` respondem `404` quando o ID não existe. A exclusão de registros iniciais permanece após reinicialização: o seed é aplicado somente uma vez por ledger.

## Carga inicial

Um ledger novo recebe 34 registros iniciais de novembro de 2026. Essa carga é aplicada uma única vez por ledger e não é baixada nem atualizada automaticamente. Cada linha continua sendo um registro CRUD normal: pode ser consultada, substituída ou excluída, e a exclusão permanece após reinicialização. A API não valida esses tickers contra a cobertura COTAHIST.

Consulte `/openapi.json` para os schemas, respostas e parâmetros completos. O Swagger UI em `/docs` carrega o mesmo contrato.
