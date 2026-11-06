# 05 — Publicar consulta histórica REST e MCP com cobertura explícita

Status: completed

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

REST e MCP stdio com serviços compartilhados foram qualificados para os recursos habilitados no 0.0.1, incluindo consulta e proveniência COTAHIST com dados de teste, OpenAPI, autenticação e MCP real com role read-only. Não se exige dataset real publicado para o release do software. Entrega de dados reais a terceiros depende da elegibilidade da issue 01.

## Comments

- 05/11/2026 — Referência nas seções 13–16 e 22 do [PRD](../b3_prd.md). Exposição a terceiros depende de finalidade elegível, mesmo com Bearer.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Testes cobrem auth read/admin, OpenAPI, paginação, revisão, precisão e proveniência; MCP roda como processo real com role read-only e rejeita mutações. Rotas de detalhe de instrumento, histórico de identifiers e TSL preservam lineage e não fazem fallback entre famílias.
- 06/11/2026 — Smoke da API na imagem de qualificação, build Docusaurus e suíte REST/MCP/PostgreSQL passaram. A API sobre base restaurada retornou a mesma revisão e proveniência do backup; registro B3 permanece `deferred` no catálogo.
