---
title: Introdução
sidebar_position: 1
---

# Tucano B3

O Tucano B3 disponibiliza dados históricos de mercado derivados dos arquivos oficiais COTAHIST da B3. Esta documentação descreve também a consulta de ações e units verificadas, com seus identificadores e preços de fechamento nominais.

Todas as operações de mercado exigem autenticação configurada no serviço. Use `Authorization: Bearer <token>` ou `X-API-Key: <token>`. Health checks, Swagger UI e OpenAPI seguem as exceções publicadas pelo serviço.

Para consultar o contrato legível por máquina, acesse `/openapi.json`. O Swagger UI em `/docs` apresenta o mesmo contrato e executa chamadas sujeitas à autenticação de cada rota.

Comece por [endpoints de ações e units](./api-endpoints/equities.md), [schemas de resposta](./schemas/equities.md) e [cobertura e atualização](./operations/equity-data.md).
