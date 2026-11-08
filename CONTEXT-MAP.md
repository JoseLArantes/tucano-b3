# Mapa de contextos

Este mapa localiza o vocabulário canônico do projeto. O [PRD](docs/b3_prd.md) define requisitos; os glossários definem termos, sem declarar capacidades implementadas.

## Contextos documentados

| Contexto | Abrangência | Glossário |
| --- | --- | --- |
| Histórico de mercado B3 | Instrumentos, identificadores temporais, registros de negociação, fontes, versões, proveniência, operação e cobertura | [Histórico de mercado](docs/domain/CONTEXT.md) |

O contexto atualmente documentado é compartilhado por aquisição, persistência, REST e MCP. Seus grupos de termos não implicam novos serviços ou módulos. Criar contextos adicionais apenas quando uma fronteira de domínio real exigir vocabulário próprio, incluindo então seu `CONTEXT.md` neste mapa.

## Referências e decisões

- [PRODUCT.md](PRODUCT.md): propósito e princípios do produto.
- [AGENTS.md](AGENTS.md): regras de trabalho e validação.
- [PRD de produto](docs/b3_prd.md): escopo e comportamento-alvo.
- [Pesquisa de fontes](docs/RESEARCH.md): evidências, datas e limitações da pesquisa.
- [Plano de revisão e migração](docs/19-plan.md): avaliação do código existente.
- [ADR 0001](docs/adr/0001-revision-pinned-cotahist-export.md): contrato NDJSON, cache derivada e revisão fixa para exportações COTAHIST longas.
- [Backlog](docs/issues/README.md): prontidão, dependências e critérios de aceite.
- [Documentação Docusaurus](docusaurus/README.md): site público de guias para consumidores e operadores, alinhado ao OpenAPI gerado.

Decisões globais efetivamente tomadas devem ser registradas em `docs/adr/` quando justificadas; decisões locais ficam junto ao contexto correspondente. Não há ADR novo criado apenas pela promoção dos documentos.
