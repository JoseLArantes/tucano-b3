---
title: Schema das séries de índices
---

# Série diária dos índices

As rotas reutilizam o envelope padrão `data` e `meta`. Cada ponto histórico contém:

| Campo | Significado |
| --- | --- |
| `data` | Data do pregão em `AAAA-MM-DD` |
| `codigo` | Código do índice (`IBOV`, `IDIV` ou `IFIX`) |
| `nome` | Nome publicado para o índice |
| `pre_ult` | Fechamento do índice em pontos, com duas casas decimais |
| `var_pct` | Variação percentual perante o pregão anterior disponível |
| `pre_abr`, `pre_max`, `pre_min`, `pre_med` | Nulos, pois não constam no CSV anual |
| `vol_tot`, `num_neg`, `qtd_tot` | Nulos, pois não constam no CSV anual |

O ponto mais recente usa os mesmos campos e também pode apresentar `var_pct` nula quando houver apenas uma observação disponível. A listagem histórica inclui paginação em `paginacao`. Consulte `/openapi.json` para o schema completo e os parâmetros.
