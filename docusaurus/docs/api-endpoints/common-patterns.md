---
title: Padrões da API
---

# Padrões comuns da API REST

## Base e contrato legível por máquina

O namespace de domínio é /api/v1/b3. O contrato OpenAPI 3.1 é gerado pelo binário e servido em GET /openapi.json. GET /docs redireciona com 303 para /docs/, onde a Swagger UI está ligada a esse mesmo documento e oferece “Try it out” para o ambiente em execução. O Docusaurus é o guia narrativo separado.

Rotas de domínio e operação exigem Bearer. GET /healthz, GET /api/v1/health, GET /openapi.json, GET /docs e os assets GET sob /docs/ são exceções públicas explícitas. Tornar a UI pública não torna públicas as operações que ela chama; elas continuam com as permissões descritas na referência de cada endpoint.

## Paginação

Listagens usam limit padrão 100, mínimo 1, máximo 1000 e offset padrão 0, não negativo. Os resultados têm ordenação determinística, incluindo desempate por chave única quando a lista precisa ser estável. Valores acima do limite não devem ser presumidos como aceitos: cada filtro valida o valor e o OpenAPI por operação é a referência exata.

Filtros são fechados: parâmetros desconhecidos ou combinações inválidas retornam 400. Datas seguem YYYY-MM-DD. Intervalos de pregão incluem os extremos.

## Revisão e precisão

Respostas COTAHIST contêm data_revision. Para percorrer páginas sem trocar de conjunto, envie o mesmo UUID em cada pedido. Números financeiros e quantidades são strings decimais. Não os converta para IEEE-754 antes de cálculos de auditoria.

Cada cotação inclui coverage, identity e provenance. include_internal=true pode acrescentar metadados internos seguros em rotas que aceitem esse parâmetro; o parâmetro não eleva permissões.

## Erros

Respostas de erro tipadas usam code, message e request_id. A mensagem é controlada e não deve expor SQL, token ou caminho privado. Códigos usuais:

| HTTP | Uso |
| --- | --- |
| 400 | Filtro, paginação, identificador ou corpo inválido |
| 401 | Bearer ausente ou inválido |
| 403 | Token sem autorização administrativa ou fonte/finalidade sem elegibilidade |
| 404 | Identificador de recurso ou revisão não encontrado |
| 409 | Conflito de escopo, conflito idempotente ou série ambígua |
| 500 | Falha interna ou dependência de consulta |
| 503 | Banco, revisão ou configuração de autenticação indisponível |

Uma resposta 202 significa que uma intenção administrativa foi persistida. Não significa que o download, parsing, publicação ou rebuild terminou.

## Operações e schemas

As páginas de [COTAHIST](cotahist.md), [cadastro](instruments.md) e [operação](operations.md) descrevem métodos, parâmetros e permissões por rota. As páginas de [schemas comuns](../schemas/common.md), [COTAHIST](../schemas/cotahist.md), [cadastro](../schemas/registry.md) e [operação](../schemas/operations.md) detalham campos. Em caso de divergência, verifique o OpenAPI servido pelo mesmo build do binário usado pelo ambiente.
