---
title: Saúde e contrato OpenAPI
---

# Saúde e contrato OpenAPI

## GET /healthz

Checagem pública de liveness do processo HTTP. Retorna um objeto com healthy. Não consulta banco, não mede cobertura de dados e não executa manutenção.

## GET /api/v1/health

Checagem pública de readiness para PostgreSQL por uma consulta simples. Resposta saudável não significa que exista corpus ou revisão COTAHIST publicada. Dependência indisponível retorna 503 com uma mensagem segura.

## GET /openapi.json

Entrega o OpenAPI 3.1 gerado pelo mesmo binário, incluindo rotas, schemas e esquema Bearer. Use esse JSON para gerar clientes e verificar o conjunto exato de operações do release. É uma exceção GET pública e não expõe dados de mercado.

## GET /docs

Redireciona com HTTP 303 para /docs/. A interface interativa Swagger UI pública é servida em GET /docs/ com HTTP 200. Ela carrega o documento OpenAPI do próprio serviço em /openapi.json; assim, métodos, parâmetros, schemas e respostas exibidos correspondem ao binário em execução. “Try it out” envia chamadas reais para esse ambiente: a UI não ignora Bearer, autorização administrativa, elegibilidade ou validação de parâmetros. Use o token adequado no botão de autorização do Swagger UI e evite ambientes compartilhados para credenciais sensíveis.

O OpenAPI JSON permanece disponível diretamente em /openapi.json para geração de clientes e ferramentas automatizadas. A interface Swagger é apenas outra forma de explorar e executar o contrato. Nenhuma das duas substitui o site Tucano B3 Docs, que contém guias narrativos para consumidores e operadores.

Os assets estáticos da interface sob /docs/ também são públicos por GET. Outros métodos HTTP continuam sujeitos ao middleware de autenticação.

## Segurança

Essas exceções são explícitas no middleware e limitadas aos métodos GET. Não inferir que qualquer endpoint de ingestão, operação ou consulta tenha a mesma exceção.
