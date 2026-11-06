# 05 — Publicar consulta histórica REST e MCP com cobertura explícita

Status: needs-info

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

## Prontidão para execução

Aguardar baseline/contratos da issue 02 e núcleo publicável da issue 04; exposição externa depende da finalidade elegível da issue 01. O status deve mudar para `ready-for-agent` somente após registrar a evidência que libera a entrega completa.

## Comments

- 05/11/2026 — Referência nas seções 13–16 e 22 do [PRD](../b3_prd.md). Exposição a terceiros depende de finalidade elegível, mesmo com Bearer.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
