---
title: Proventos
---

# Proventos

O recurso mantém registros de `Dividendo`, `JCP` e `Rendimento` no ledger SQLite. Cada lançamento inclui ticker, valor por ação ou cota, datas disponíveis e `obs` para fonte e observações. A API preserva valores em escala decimal exata de seis casas e não interpreta estes registros como uma série oficial da B3. A proveniência informada em `obs` é armazenada literalmente; a API não consulta nem valida a fonte.

As rotas exigem Bearer ou API Key quando a autenticação está habilitada. `POST`, `PUT` e `DELETE` retornam `503` se não houver credencial configurada. Sucessos usam `{ data, meta }`; erros usam `{ error }`.

## Operações

| Método | Rota | Uso |
| --- | --- | --- |
| `GET` | `/api/v1/proventos` | Lista registros com filtros e paginação. |
| `POST` | `/api/v1/proventos` | Cadastra um provento (`201`) ou retorna o existente (`200`). |
| `POST` | `/api/v1/proventos/upload` | Importa um lote de registros de um CSV (`200`). |
| `GET` | `/api/v1/proventos/yearly` | Soma proventos de um ticker por ano civil da data-com. |
| `GET` | `/api/v1/proventos/period` | Soma proventos por ticker em um intervalo de data-com. |
| `GET` | `/api/v1/proventos/series` | Retorna séries diárias por ticker para gráficos. |
| `GET` | `/api/v1/proventos/{id}` | Consulta um registro por identificador. |
| `PUT` | `/api/v1/proventos/{id}` | Substitui os campos de negócio do registro. |
| `DELETE` | `/api/v1/proventos/{id}` | Exclui o registro persistido. |

## Consulta e filtros

`GET /api/v1/proventos` aceita os parâmetros opcionais `ticker`, `tickers`, `tipo`, `data_pagamento_desde`, `data_pagamento_ate`, `pagina` e `por_pagina`.

| Parâmetro | Formato | Padrão/limite | Comportamento |
| --- | --- | --- | --- |
| `ticker` | texto alfanumérico | 1–10 caracteres | Comparação exata após normalização para maiúsculas. |
| `tickers` | lista separada por vírgula | 1–100 códigos | Retorna registros para qualquer código da lista. Mutuamente exclusivo com `ticker`; códigos duplicados são removidos. |
| `tipo` | enum | `Dividendo`, `JCP`, `Rendimento` | Filtra pela categoria do registro. |
| `data_pagamento_desde` | `AAAA-MM-DD` | opcional | Limite inferior inclusivo. Exclui registros sem data de pagamento. |
| `data_pagamento_ate` | `AAAA-MM-DD` | opcional | Limite superior inclusivo. Exclui registros sem data de pagamento. |
| `pagina` | inteiro | `1`, mínimo `1` | Página baseada em 1. Valores menores são normalizados para 1. |
| `por_pagina` | inteiro | `50`, mínimo `1`, máximo `1000` | A API limita valores fora do intervalo permitido. |

Quando os dois limites de pagamento são informados, `desde` deve anteceder ou ser igual a `ate`. A ordem dos resultados é data de pagamento crescente, ticker, tipo e ID; registros sem data de pagamento ficam no final. A resposta inclui `data.proventos` e `data.paginacao` com `total`, `pagina`, `por_pagina` e `total_paginas`.

`ticker` filtra um único código e permanece disponível por compatibilidade. Para filtrar vários, use `tickers=PETR4,VALE3`; valores são aparados, normalizados para maiúsculas e deduplicados. Os demais filtros são combinados com a seleção de tickers, e a paginação se aplica ao conjunto resultante.

```bash
curl -s -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  'http://localhost:8080/api/v1/proventos?tickers=PETR4,VALE3&tipo=JCP&data_pagamento_desde=2026-11-01&data_pagamento_ate=2026-11-30' | jq
```

## Relatórios por data-com

Os relatórios somam Dividendo, JCP e Rendimento por ação ou cota em reais. Cada lançamento entra na soma pela `data_com`, mesmo quando o pagamento ocorre em outro intervalo. Lançamentos sem data-com ficam fora. Esses valores vêm dos registros cadastrados.

| GET | Parâmetros obrigatórios | Resultado em `data` |
| --- | --- | --- |
| `/api/v1/proventos/yearly` | `ticker` e `ano`, de 1 a 9999 | Um objeto com `ticker`, `ano` e `proventos_on_year`. |
| `/api/v1/proventos/period` | `ticker` ou `tickers`, `desde` e `ate` | Uma lista com `ticker` e `proventos_on_period` para cada código. |
| `/api/v1/proventos/series` | `ticker` ou `tickers`, `desde` e `ate` | Uma lista com `ticker` e `series` para cada código. |

Use `desde` e `ate` no formato `AAAA-MM-DD`, com `desde` menor ou igual a `ate`. Os limites são inclusivos. Em `tickers`, use de 1 a 100 códigos separados por vírgula. Cada código contém de 1 a 10 caracteres alfanuméricos. Após remover os espaços externos e normalizar para maiúsculas, os relatórios retornam cada ticker distinto na ordem em que aparece pela primeira vez.

Os relatórios retornam `200` com `"0.000000"` para totais sem registros e `[]` para a série do ticker sem registros. Os dias com lançamentos aparecem em ordem crescente como pontos `{date_ms, valor}`. Os dias sem lançamentos são omitidos. Os relatórios não são paginados. Consulte os [schemas de relatórios](../schemas/proventos.md#relatórios).

```bash
base='http://localhost:8080/api/v1/proventos'
curl -fsS -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  "$base/yearly?ticker=TAEE3&ano=2026"
curl -fsS -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  "$base/period?tickers=TAEE3,PETR4&desde=2026-08-14&ate=2026-08-21"
curl -fsS -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  "$base/series?tickers=TAEE3,PETR4&desde=2026-08-14&ate=2026-08-21"
```

Parâmetros ausentes, inválidos ou desconhecidos retornam `400`. Credenciais ausentes ou inválidas retornam `401` quando a autenticação está habilitada. O limite de requisições retorna `429`. Falhas internas de consulta retornam `500`.

## Importar proventos por CSV no Swagger

1. Abra `/docs` na instância da API e forneça um token Bearer ou uma chave de API em **Authorize**.
2. No grupo **proventos**, abra **POST /proventos/upload** e clique em **Try it out**.
3. Selecione o CSV no campo `file` e clique em **Execute**.
4. Consulte `data.inseridos`, `data.atualizados` e `data.ignorados` para conferir o resultado.

Prepare um único CSV em UTF-8, separado por vírgulas. O cabeçalho é obrigatório e aceita colunas em qualquer ordem. Informe `ticker`, `tipo` e `valor`. Acrescente `data_com`, `data_pagamento` e `obs` conforme os dados disponíveis. Use ponto decimal e datas em `AAAA-MM-DD`. Coloque entre aspas um texto que contenha vírgulas.

```csv
ticker,tipo,valor,data_com,data_pagamento,obs
TSTU11,Rendimento,0.123456,2026-10-30,2026-11-10,"Comunicado do fundo, outubro"
TSTU3,Dividendo,0.230000,2026-11-01,,
TSTU4,JCP,0.120000,,,
```

O limite é 1 MiB e 10.000 registros. Um arquivo inválido retorna `400` sem gravar registros. A resposta aponta a linha quando o erro está em um registro. Arquivos acima do limite de tamanho retornam `413`. Credenciais ausentes ou inválidas retornam `401` quando a autenticação está habilitada. Uma instância sem credencial configurada retorna `503`. Limites de requisição e falhas internas retornam `429` e `500`, respectivamente.

O serviço identifica o registro pelo ticker normalizado, tipo, valor exato e ambas as datas. Em registros existentes, somente `obs` pode mudar. Quando a identidade se repete no arquivo, vale a última linha. Consulte os [schemas de importação](../schemas/proventos.md#importação-csv).

## Criar, substituir e excluir

Envie o corpo fechado `ProventoRequest` com `ticker`, `tipo`, `valor`, `data_com`, `data_pagamento` e `obs`. Campos desconhecidos são rejeitados. `tipo` aceita somente `Dividendo`, `JCP` ou `Rendimento`. `valor` é uma string decimal positiva em reais com até seis casas, sem sinal, separador de milhar ou arredondamento; o máximo equivale a `DECIMAL(14,6)`. A API responde sempre com seis casas. `data_com` e `data_pagamento` aceitam `null`; quando ambas existem, a data-com não pode ser posterior ao pagamento. Datas inválidas ou fora de `AAAA-MM-DD` são rejeitadas. `ticker` é normalizado para maiúsculas e aceita de 1 a 10 caracteres alfanuméricos. `obs` aceita até 255 caracteres e deve registrar a origem quando conhecida.

O `POST` retorna `201` para um registro novo e `200` para um registro existente, mantendo seu ID e atualizando apenas `obs` se necessário. O `PUT` substitui os campos de negócio e retorna `409` se a nova identidade já pertence a outro ID. O `DELETE` remove permanentemente o registro.

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

`PUT` e `DELETE` respondem `404` quando o ID não existe. A exclusão de registros iniciais permanece após reinicialização: o seed é aplicado somente uma vez por ledger.

## Carga inicial

Um ledger novo recebe 34 registros iniciais de novembro de 2026. Essa carga é aplicada uma única vez por ledger e não é baixada nem atualizada automaticamente. Cada linha continua sendo um registro CRUD normal: pode ser consultada, substituída ou excluída, e a exclusão permanece após reinicialização. A API não valida esses tickers contra a cobertura COTAHIST.

Consulte `/openapi.json` para os schemas, respostas e parâmetros completos. O Swagger UI em `/docs` carrega o mesmo contrato.
