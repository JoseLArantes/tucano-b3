---
title: Schema de proventos
---

# Schema de proventos

As respostas de CRUD são envolvidas por `{ data, meta }`; erros seguem `{ error }`. Os objetos são fechados e rejeitam campos desconhecidos.

## Provento

| Campo | Tipo | Significado |
| --- | --- | --- |
| `id` | inteiro de 64 bits | Identificador estável gerado pelo SQLite. |
| `ticker` | string (1–10) | Código alfanumérico normalizado para maiúsculas. |
| `tipo` | enum string | `Dividendo`, `JCP` ou `Rendimento`. |
| `valor` | decimal textual | Valor unitário em reais com seis casas na resposta; até seis casas na entrada. |
| `data_com` | data ISO ou `null` | Data-com quando publicada pela fonte. |
| `data_pagamento` | data ISO ou `null` | Data prevista de pagamento quando publicada. |
| `obs` | string de até 255 caracteres ou `null` | Fonte e observações associadas ao lançamento. |
| `criado_em` | string ISO 8601 UTC | Instante de criação. |
| `atualizado_em` | string ISO 8601 UTC | Instante da última substituição. |

Valores são armazenados como micro-reais inteiros no ledger, sem conversão por ponto flutuante. A entrada rejeita valor zero/negativo, mais de seis casas e valores acima do limite equivalente a `DECIMAL(14,6)`. O JSON usa string para conservar essa precisão.

O corpo de `POST` e `PUT` contém `ticker`, `tipo`, `valor`, `data_com`, `data_pagamento` e `obs`. Os dois campos de data podem ser nulos. Quando ambos são informados, `data_com` deve ser anterior ou igual a `data_pagamento`.

## Lista

`GET /api/v1/proventos` retorna:

- `proventos`: lista paginada de objetos Provento.
- `paginacao`: `total`, `pagina`, `por_pagina` e `total_paginas`.

Filtros opcionais: `ticker` para um único código, `tickers` para uma lista separada por vírgula (máximo de 100), `tipo`, `data_pagamento_desde` e `data_pagamento_ate`. `ticker` e `tickers` são mutuamente exclusivos. Intervalos de pagamento são inclusivos. A ordenação é determinística: data de pagamento, ticker, tipo e ID; valores sem data ficam no final.

O ledger recebe um conjunto inicial de registros de novembro de 2026 uma única vez. Para qualquer registro, `obs` contém somente a informação de proveniência fornecida no cadastro; a API não verifica sua veracidade. Proventos cadastrados não são apresentados como série oficial da B3.

## Importação CSV

`POST /api/v1/proventos/upload` usa `multipart/form-data` com um campo binário obrigatório chamado `file`. O arquivo tem cabeçalho e de 1 a 10.000 registros em UTF-8, com limite de 1 MiB. As colunas obrigatórias são `ticker`, `tipo` e `valor`, com as mesmas regras do cadastro individual. As colunas `data_com`, `data_pagamento` e `obs` são opcionais. O serviço interpreta célula opcional vazia como nula e rejeita cabeçalhos repetidos ou desconhecidos.

A identidade combina `ticker` normalizado, `tipo`, `valor` exato em micro-reais, `data_com` e `data_pagamento`. Datas nulas coincidem entre si. Quando só `obs` difere, o mesmo registro é atualizado.

A resposta `200` usa o envelope `{ data, meta }`. Exemplo de `data`:

```json
{"recebidos": 4, "inseridos": 2, "atualizados": 1, "ignorados": 1}
```

| Campo | Tipo | Significado |
| --- | --- | --- |
| `recebidos` | inteiro | Quantidade de registros válidos lidos, sem contar o cabeçalho. |
| `inseridos` | inteiro | Quantidade de registros novos gravados na transação. |
| `atualizados` | inteiro | Quantidade de observações alteradas em registros existentes. |
| `ignorados` | inteiro | Linhas substituídas por uma ocorrência posterior no arquivo ou já persistidas sem alteração. |

## Relatórios

Os três relatórios usam o envelope `{ data, meta }`. As somas em reais por ação ou cota incluem todos os tipos e são strings com seis casas decimais. Os exemplos abaixo mostram o conteúdo de `data`.

**Total anual**, em `GET /api/v1/proventos/yearly`.

```json
{"ticker": "TAEE3", "ano": 2026, "proventos_on_year": "0.200100"}
```

**Total por período**, em `GET /api/v1/proventos/period`.

```json
[
  {"ticker": "TAEE3", "proventos_on_period": "0.200100"},
  {"ticker": "PETR4", "proventos_on_period": "0.674100"}
]
```

**Série diária**, em `GET /api/v1/proventos/series`.

```json
[
  {"ticker": "TAEE3", "series": [{"date_ms": 1786665600000, "valor": "0.200100"}]}
]
```

Cada `date_ms` é um inteiro de 64 bits que representa a data-com à meia-noite UTC em milissegundos desde o Unix epoch. Em gráficos baseados em data civil, formate o eixo em UTC. Cada `valor` é a soma dos lançamentos daquela data-com. Séries não preenchem dias sem lançamentos.
