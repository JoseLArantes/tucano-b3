# Backlog de desenvolvimento

Os documentos desta pasta são issues aceitas para acompanhamento do [PRD](../b3_prd.md), com critérios de aceite e dependências explícitas. Aceitação no backlog não significa que todas estejam liberadas para implementação imediata ou que seus requisitos já estejam entregues.

O [plano de marcos de desenvolvimento](../prd_dev_milestones.md) organiza as entregas G0–G7 a partir da base atual, incluindo trabalho concomitante e critérios de lançamento. Ele não altera automaticamente a triagem das issues abaixo.

Triagem revisada em **06/11/2026**. A [issue 09](09-prd-alignment-review.md) registra a conclusão da revisão histórica na baseline `fdc804f`; desde então, o plano [0.0.1](../0.0.1-plan.md) integrou o modelo, aquisição, núcleo, queries e coordenação em testes sintéticos e PostgreSQL. Os estados abaixo continuam refletindo o aceite completo por issue: evidência de elegibilidade, corpus de produção, homologação e operação ainda impedem seu encerramento.

## Prontidão

| Issue | Marco | Status | Condição de execução |
| --- | --- | --- | --- |
| [01 — Viabilidade de fontes](01-source-viability.md) | G0 | `needs-info` | Falta evidência de permissão e custo zero para aquisição, retenção e entrega a terceiros; sem corpus oficial congelado |
| [02 — Domínio e contratos](02-domain-and-contracts.md) | G1 | `needs-info` | Modelo e contratos implementados; natural keys e semântica precisam de homologação com corpus permitido |
| [03 — Aquisição e artefatos](03-artifact-acquisition.md) | G2 | `needs-info` | Pipeline técnico implementado; homologação dos canais/fontes externos depende de 01 |
| [04 — Núcleo histórico](04-historical-core.md) | G3 | `needs-info` | Pipeline integrado com fixtures; falta ingestão/reconciliação do corpus permitido e critérios integrados de aceite |
| [05 — Produto de consulta](05-query-product.md) | G4 | `needs-info` | REST/MCP e contratos implementados; falta smoke de release e dataset publicado/corpus homologado |
| [06 — Operação e recuperação](06-operations.md) | G5 | `needs-info` | Jobs, discovery e recuperação implementados; faltam medições, budgets e qualificação de operação de release |
| [07 — Boletins de preços](07-price-reports.md) | G6 | `needs-info` | Plataforma 02–06 e corpus/elegibilidade próprios em 01 |
| [08 — Índices e carteiras](08-indices-and-portfolios.md) | G7 | `needs-info` | Plataforma 02–06 e corpus/elegibilidade próprios em 01 |
| [09 — Revisão de aderência](09-prd-alignment-review.md) | R0–R6 | `completed` | Conclusão registrada em 06/11/2026: baseline OpenAPI 3.1, matriz de aderência e plano de migração; não encerra os marcos de produto |

`needs-info` identifica um pré-requisito ainda não demonstrado e não afirma que o código correspondente esteja ausente. Trabalho de implementação e correção pode avançar quando a evidência de aceitação estiver disponível; ausência de autorização/corpus não pode ser substituída por fixture sintética.

## Ordem de trabalho

1. Manter os resultados de 09 como baseline histórica; os detalhes de implementação corrente ficam no [plano 0.0.1](../0.0.1-plan.md) e nesta matriz de marcos.
2. Resolver G0 e congelar o corpus antes de qualquer aquisição ou entrega externa.
3. Homologar chaves, formatos, calendário e reconciliação nos fluxos implementados usando o corpus autorizado.
4. Concluir smoke, benchmark, budgets e restore com os serviços de release integrados.
5. Liberar G6/G7 somente após critérios de plataforma e evidência próprios de cada fonte.

A revisão 09 pode propor subtarefas menores e reutilizar estas issues. Não duplicar escopos nem reaproveitar números; a próxima issue nova começa em `10`.

## Manutenção

- Atualizar o status no arquivo e neste índice na mesma entrega.
- Preservar critérios de aceite, motivos das dependências e histórico em `## Comments`.
- Usar `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` ou `wontfix` conforme o caso. Não trocar ausência de evidência por suposição de sucesso.
- Rascunhos e relatórios intermediários podem permanecer em `.scratch/`; issues aceitas ficam nesta pasta. O [PRD](../b3_prd.md) continua sendo o único PRD do produto.

## Comments

- 06/11/2026 — Backlog promovido de `.scratch/b3-historical-data/issues` após aceitação dos documentos. Dependências ainda abertas foram preservadas; criada a issue 09 para executar a avaliação antes das mudanças de código.
- 06/11/2026 — Issue 09 concluída com sucesso (fases R0–R6 do plano 19). Suíte de testes endurecida para 35 testes determinísticos sem skips no PostgreSQL 16; contratos OpenAPI 3.1 e envelopes inventariados; matriz de aderência (`assessment.md`) e plano de migração (`migration.md`) consolidados sob `.scratch/prd-alignment/`. Próximos passos seguem a ordem de trabalho a partir das issues 01 e 02.
- 06/11/2026 — Marcos G0–G7 detalhados em `docs/prd_dev_milestones.md`; ordem de trabalho atualizada para partir da revisão registrada como concluída. Status das issues preservados; evidências posteriores e propostas de migração devem ser reconciliadas antes da próxima implementação.
- 06/11/2026 — Implementação 0.0.1 integrada com 65 testes passando em PostgreSQL 16.15 isolado; G0 segue `needs-info` e o lançamento externo permanece bloqueado. Ver matriz atual no plano 0.0.1 e marcos de desenvolvimento.
