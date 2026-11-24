# Proventos

O serviço oferece CRUD autenticado para `Dividendo`, `JCP` e `Rendimento`, persistido no ledger SQLite. Endpoints disponíveis:

- `GET /api/v1/proventos` — filtros por ticker único ou lista de tickers, tipo e intervalo inclusivo de data de pagamento, com paginação padrão de 50 e máximo de 1000.
- `POST /api/v1/proventos` — cria registro (`201`).
- `GET /api/v1/proventos/{id}` — consulta registro.
- `PUT /api/v1/proventos/{id}` — substitui campos do registro.
- `DELETE /api/v1/proventos/{id}` — remove registro.

Sucessos usam o envelope `{ data, meta }`. Os campos são `ticker`, `tipo`, `valor`, `data_com`, `data_pagamento` e `obs`. Datas são ISO (`AAAA-MM-DD`) ou nulas. `valor` é enviado como decimal textual positivo em reais, com até seis casas, e sempre retorna formatado com seis casas. `obs` deve conservar fonte e observações (até 255 caracteres). Não há deduplicação automática; cada registro tem ID estável para permitir a gestão explícita.

Um ledger novo recebe uma única carga de 34 registros de novembro de 2026. A carga não é obtida nem atualizada automaticamente. Os registros continuam editáveis e removíveis pelo CRUD; uma exclusão não é revertida depois de reiniciar o serviço.

Para consultar vários códigos de uma vez, informe `tickers` como uma lista separada por vírgulas, com no máximo 100 tickers. O parâmetro `tickers` é mutuamente exclusivo com o filtro legado `ticker`; espaços ao redor de cada código são ignorados, os códigos são normalizados para maiúsculas e duplicatas são removidas. Os resultados seguem a ordenação e paginação padrão.

```bash
curl -s -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  'http://localhost:8080/api/v1/proventos?tickers=PETR4,VALE3&pagina=1&por_pagina=100' | jq
```

Os registros não constituem uma série oficial da B3. O campo `obs` guarda a proveniência fornecida com cada registro, sem validação automática pela API.

## Relatórios por data-com

| Rota GET | Consulta | Resultado em `data` |
| --- | --- | --- |
| `/api/v1/proventos/yearly` | `ticker`, `ano` | Objeto com `ticker`, `ano` e `proventos_on_year`. |
| `/api/v1/proventos/period` | `ticker` ou `tickers`, `desde`, `ate` | Lista com um `proventos_on_period` por ticker. |
| `/api/v1/proventos/series` | `ticker` ou `tickers`, `desde`, `ate` | Lista com uma série de pontos `{date_ms, valor}` por ticker. |

Os relatórios somam Dividendo, JCP e Rendimento em reais por ação ou cota. O filtro usa `data_com` e inclui as duas datas do intervalo. Lançamentos sem data-com ficam fora. Em `period` e `series`, informe até 100 códigos separados por vírgula em `tickers` ou um código em `ticker`. A resposta mantém a ordem da primeira ocorrência de cada ticker normalizado. Relatórios não são paginados. Sem registros, o total é `"0.000000"` e a série é vazia.

```bash
curl -fsS -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  'http://localhost:8080/api/v1/proventos/yearly?ticker=TAEE3&ano=2026'

curl -fsS -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  'http://localhost:8080/api/v1/proventos/period?tickers=TAEE3,PETR4&desde=2026-08-14&ate=2026-08-21'

curl -fsS -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  'http://localhost:8080/api/v1/proventos/series?tickers=TAEE3,PETR4&desde=2026-08-14&ate=2026-08-21'
```

Cada ponto usa a data-com à meia-noite UTC em milissegundos desde o Unix epoch. `valor` e os totais dos relatórios são strings decimais exatas de seis casas.

Consulte a referência no `openapi.json` e os guias Docusaurus de [endpoints](../docusaurus/docs/api-endpoints/proventos.md) e [schemas](../docusaurus/docs/schemas/proventos.md).
