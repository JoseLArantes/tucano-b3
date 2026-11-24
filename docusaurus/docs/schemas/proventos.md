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

Filtros opcionais: `ticker`, `tipo`, `data_pagamento_desde` e `data_pagamento_ate`. Intervalos de pagamento são inclusivos. A ordenação é determinística: data de pagamento, ticker, tipo e ID; valores sem data ficam no final.

O ledger recebe um conjunto inicial de registros de novembro de 2026 uma única vez. Para qualquer registro, `obs` contém somente a informação de proveniência fornecida no cadastro; a API não verifica sua veracidade. Proventos cadastrados não são apresentados como série oficial da B3.
