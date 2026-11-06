---
title: MCP analítico
---

# Servidor MCP analítico

O MCP usa stdio e ferramentas somente leitura. Ele compartilha QueryService com a API REST, aplica elegibilidade para finalidade MCP e usa uma credencial PostgreSQL de leitura. O processo escreve mensagens MCP no stdout; logs de diagnóstico devem ficar em stderr.

## Inicialização

O cliente deve iniciar o executável com o subcomando mcp, trocar initialize, enviar notifications/initialized e então consultar tools/list e tools/call. O servidor aceita versões MCP 2024-11-05, 2025-03-26 e 2025-06-18; versão desconhecida usa fallback compatível 2024-11-05.

## Ferramentas publicadas

| Ferramenta | Uso |
| --- | --- |
| healthcheck | Confirma acesso analítico read-only ao PostgreSQL |
| search_instruments | Busca fotografias cadastrais publicadas e qualificadas |
| get_ohlcv_series | Consulta cotação COTAHIST nominal; exige filtros para resolver séries ambíguas |
| get_instrument_coverage | Consulta fatos e cobertura com os filtros de cotação |
| get_dataset_diagnostics | Retorna diagnóstico da revisão publicada elegível |

Filtros são schemas fechados. Cotações retornam revisão, proveniência, estado de identidade e cobertura. get_ohlcv_series não ajusta preços nem preenche pregões ausentes.

## Limites

Não há ferramenta para ingestão, replay, rebuild, publicação, rollback, SQL arbitrário ou mutação de estado. MCP segue as mesmas limitações de elegibilidade, precisão, identidade temporal e preços nominais da REST.
