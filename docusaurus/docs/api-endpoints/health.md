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

Página mínima pública que apresenta o contrato e liga para /openapi.json. Ela não é uma UI interativa do Swagger e não substitui o site Tucano B3 Docs.

## Segurança

Essas exceções são explícitas no middleware e limitadas aos métodos GET. Não inferir que qualquer endpoint de ingestão, operação ou consulta tenha a mesma exceção.
