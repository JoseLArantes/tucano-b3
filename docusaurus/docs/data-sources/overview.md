---
title: Visão geral das fontes
---

# Fontes oficiais B3

O produto aceita apenas publicações oficiais B3 dentro do escopo aprovado. As famílias mantêm layouts, granularidades, unidades, temporalidade e linhagens próprias.

| Família | Conteúdo | Tratamento documental |
| --- | --- | --- |
| COTAHIST diário, mensal e anual | Cotações históricas nominais | Núcleo de consultas e revisões do release documentado |
| BVBG.028.02 / InstrumentsConsolidated | Dados cadastrais e definições de instrumentos | Fotografia de cadastro; não transforma ticker atual em identidade histórica |
| Tradable Security List | Instrumentos negociáveis e seus qualificadores | Família própria; o endpoint dedicado não faz fallback a BVBG.028 |
| BVBG.086.01 | PriceReport completo | Contrato e linhagem separados de outras famílias de preço |
| BVBG.186.01 | Simplified Price Report | Contrato e linhagem separados de BVBG.086 |
| BVBG.087.01 | IndexReport | Tipos de valor e unidades permanecem explícitos |
| Composições de índices | Componentes, pesos e referências | Data-base e período de vigência são distintos de uma observação atual |

## Escopo, implementação e disponibilidade

Há três questões diferentes:

1. **Escopo do produto**: fontes que o PRD define como aceitas.
2. **Implementação**: parsers, persistência e contratos disponíveis no código.
3. **Disponibilidade por ambiente**: arquivo cadastrado, publicação existente, fonte habilitada e elegibilidade para finalidade/canal.

Um endpoint pode existir e retornar fonte não elegível, revisão indisponível ou conjunto vazio. Consulte GET /api/v1/b3/capabilities e GET /api/v1/b3/diagnostics/coverage para o estado que o ambiente efetivamente pode expor. Não trate ausência de dados como prova de ausência de negociação.

## Linhagem e aquisição

O serviço retém arquivo bruto por hash e inventaria members em ordem determinística, inclusive entradas dentro de containers. O processamento registra parser, bytes, membros, contadores e rejeições. A API oferece inspeção administrativa dos arquivos e members para correlacionar o dado publicado ao processo.

Consulte [COTAHIST](cotahist.md) e [cadastro de instrumentos](registry.md) para os detalhes das famílias com consulta REST inicial documentada.
