---
title: Changelog da documentação
---

# Changelog

## 08/11/2026 — Descoberta configurável de COTAHIST

- `POST /api/v1/b3/discovery/scan` consulta pastas, URLs diretas oficiais B3 e buckets AWS S3 compatíveis configurados em `B3_DISCOVERY_LOCATIONS`.
- O contrato responde com resultados por local e candidatos tipados por metadados, sem baixar, ingerir ou criar pendências de canal para esses locais.
- Veja [descoberta](../operations/discovery.md), [endpoints operacionais](../api-endpoints/operations.md) e [schemas operacionais](../schemas/operations.md). Compatibilidade aditiva na resposta do endpoint existente.

## 08/11/2026 — Exportação integral COTAHIST

- Publicado `GET /api/v1/b3/cotahist/quotes/export` em NDJSON finito, com revisão pinada, checkpoints, marcador `complete`, cache privado reconstruível e suporte a gzip/ETag para artefatos prontos.
- A consulta paginada existente permanece compatível; consulte [endpoints COTAHIST](../api-endpoints/cotahist.md) e [schema COTAHIST](../schemas/cotahist.md).
- A meta de primeira cotação sob um segundo e 50 consumidores simultâneos ainda requer benchmark e não é resultado medido.

## 06/11/2026 — Primeira versão do site Docusaurus

- Criado guia de consumidores e operadores para o contrato REST inicial, fontes, schemas, autenticação, operação e MCP.
- Incluídos exemplos e tabelas alinhados às rotas e DTOs implementados no release 0.0.1.
- Documentadas explicitamente as diferenças entre o escopo do PRD, implementação e elegibilidade por ambiente.
- Sem alteração de endpoint, campo ou compatibilidade da API. O changelog de contratos continua em [docs/frontend_api_changelog.md](https://github.com/JoseLArantes/b3-tucano/blob/main/docs/frontend_api_changelog.md).
