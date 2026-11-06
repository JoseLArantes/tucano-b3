# Backlog de desenvolvimento

Os documentos desta pasta são issues aceitas para acompanhamento do [PRD](../b3_prd.md), com critérios de aceite e dependências explícitas. Aceitação no backlog não significa que todas estejam liberadas para implementação imediata ou que seus requisitos já estejam entregues.

Triagem em **06/11/2026**. A [issue 09](09-prd-alignment-review.md) é o ponto de entrada executável: revisar o código conforme o [plano 19](../19-plan.md), produzindo evidência antes de definir as alterações de implementação.

## Prontidão

| Issue | Marco | Status | Condição de execução |
| --- | --- | --- | --- |
| [01 — Viabilidade de fontes](01-source-viability.md) | G0 | `needs-info` | Confirmar elegibilidade e corpus por finalidade; preparação offline pode avançar |
| [02 — Domínio e contratos](02-domain-and-contracts.md) | G1 | `needs-info` | Revisão 09 concluída; aguarda evidência de 01 para homologação definitiva do corpus |
| [03 — Aquisição e artefatos](03-artifact-acquisition.md) | G2 | `needs-info` | Contratos de 02; aquisição externa depende de 01 |
| [04 — Núcleo histórico](04-historical-core.md) | G3 | `needs-info` | 01, 02, 03 e coordenação com 06 |
| [05 — Produto de consulta](05-query-product.md) | G4 | `needs-info` | Baseline/contratos de 02 e núcleo publicável de 04 |
| [06 — Operação e recuperação](06-operations.md) | G5 | `needs-info` | Escopos/estados de 02; desenvolvimento conjunto com 03/04 |
| [07 — Boletins de preços](07-price-reports.md) | G6 | `needs-info` | Plataforma 02–06 e corpus/elegibilidade próprios em 01 |
| [08 — Índices e carteiras](08-indices-and-portfolios.md) | G7 | `needs-info` | Plataforma 02–06 e corpus/elegibilidade próprios em 01 |
| [09 — Revisão de aderência](09-prd-alignment-review.md) | R0–R6 | `completed` | Concluída em 06/11/2026: 35 testes reais, baseline OpenAPI 3.1, matriz de aderência e plano de migração |

`needs-info` identifica um pré-requisito ainda não demonstrado, inclusive uma entrega técnica anterior; não exige que o usuário responda quando o agente puder obter a informação pelo trabalho autorizado. A preparação independente pode avançar, mas a issue completa só passa a `ready-for-agent` quando suas condições de entrada estiverem demonstradas.

## Ordem de trabalho

1. Executar 09 para estabelecer baseline, matriz de aderência e backlog de correções, sem iniciar um rewrite.
2. Avançar a preparação independente de 01; obter evidência externa antes de habilitar aquisição/distribuição dependente dela.
3. Liberar 02 após a revisão; conclusões que dependem do corpus continuam pendentes até sua homologação.
4. Decompor 03, 04 e 06 em entregas coesas com interfaces comuns. Coordenação não exige a conclusão circular de todos os marcos.
5. Liberar 05, 07 e 08 conforme a plataforma e os gates específicos estiverem comprovados.

A revisão 09 pode propor subtarefas menores e reutilizar estas issues. Não duplicar escopos nem reaproveitar números; a próxima issue nova começa em `10`.

## Manutenção

- Atualizar o status no arquivo e neste índice na mesma entrega.
- Preservar critérios de aceite, motivos das dependências e histórico em `## Comments`.
- Usar `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human` ou `wontfix` conforme o caso. Não trocar ausência de evidência por suposição de sucesso.
- Rascunhos e relatórios intermediários podem permanecer em `.scratch/`; issues aceitas ficam nesta pasta. O [PRD](../b3_prd.md) continua sendo o único PRD do produto.

## Comments

- 06/11/2026 — Backlog promovido de `.scratch/b3-historical-data/issues` após aceitação dos documentos. Dependências ainda abertas foram preservadas; criada a issue 09 para executar a avaliação antes das mudanças de código.
- 06/11/2026 — Issue 09 concluída com sucesso (fases R0–R6 do plano 19). Suíte de testes endurecida para 35 testes determinísticos sem skips no PostgreSQL 16; contratos OpenAPI 3.1 e envelopes inventariados; matriz de aderência (`assessment.md`) e plano de migração (`migration.md`) consolidados sob `.scratch/prd-alignment/`. Próximos passos seguem a ordem de trabalho a partir das issues 01 e 02.
