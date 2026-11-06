# 05 — Publicar consulta histórica REST e MCP com cobertura explícita

Status: needs-triage

Data: 05/11/2026. Marco: G4. Dependências: 01, 02 e 04.

## Problema

Consumidores precisam distinguir ausência, falha, identidade incerta e base nominal do preço, com contratos estáveis e consultas limitadas.

## Escopo

- Query services compartilhados; filtros e ordenação por família; paginação consistente por revisão quando oferecida.
- Preservar rotas e envelope v1; versionar alterações incompatíveis.
- Proveniência, qualidade e cobertura; campos internos condicionais e dados sensíveis excluídos.
- Ferramentas MCP tipadas, somente de leitura, com papéis de banco e limites adequados.
- OpenAPI completo, exemplos, README, documentação operacional/consumidor e changelog.

## Critérios de aceite

- Mesmo pedido equivalente em REST/MCP produz o mesmo significado e proveniência.
- Série ambígua exige seleção; nenhuma linha é escolhida arbitrariamente.
- Todos os decimais são strings e o aviso COTAHIST aparece nas superfícies pertinentes.
- Falha de banco não vira lista vazia; cobertura parcial é comunicada.
- MCP não migra banco, não aceita SQL arbitrário e não expõe mutações.
- Testes verificam endpoints efetivos contra OpenAPI e cliente MCP compatível.

## Comments

- 05/11/2026 — Referência nas seções 13–16 e 22 do [PRD](../../../docs/b3_prd.md). Exposição a terceiros depende de finalidade elegível, mesmo com Bearer.
