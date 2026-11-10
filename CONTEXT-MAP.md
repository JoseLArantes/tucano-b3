# Mapa de contextos

Este mapa localiza o vocabulário canônico do projeto. O [PRD](docs/prd.md) define requisitos; os glossários definem termos, sem declarar capacidades implementadas.

## Contextos documentados

| Contexto | Abrangência | Glossário |
| --- | --- | --- |
| Histórico de mercado B3 | Instrumentos, identificadores temporais, registros de negociação, fontes, versões, proveniência, operação e cobertura | [Histórico de mercado](docs/domain/CONTEXT.md) |

O contexto atualmente documentado é compartilhado por aquisição, persistência, REST e MCP. Seus grupos de termos não implicam novos serviços ou módulos. Criar contextos adicionais apenas quando uma fronteira de domínio real exigir vocabulário próprio, incluindo então seu `CONTEXT.md` neste mapa.

## Referências e decisões

- [PRODUCT.md](PRODUCT.md): propósito e princípios do produto.
- [AGENTS.md](AGENTS.md): regras de trabalho e validação.
- [PRD de produto](docs/prd.md): escopo e comportamento-alvo.
- [Documentação Docusaurus](docusaurus/README.md): site público de guias para consumidores e operadores, alinhado ao OpenAPI gerado.

Decisões globais efetivamente tomadas devem ser registradas em `docs/adr/` quando justificadas; decisões locais ficam junto ao contexto correspondente. Não há ADR novo criado apenas pela promoção dos documentos.
