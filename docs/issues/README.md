# Backlog de desenvolvimento

Os documentos desta pasta são issues aceitas para acompanhamento do [PRD](../b3_prd.md), com critérios de aceite e dependências explícitas. Aceitação no backlog não significa que todas estejam liberadas para implementação imediata ou que seus requisitos já estejam entregues.

O [plano de marcos de desenvolvimento](../prd_dev_milestones.md) organiza as entregas G0–G7 a partir da base atual, incluindo trabalho concomitante e critérios de lançamento. Ele não altera automaticamente a triagem das issues abaixo.

Triagem revisada em **06/11/2026**. A [issue 09](09-prd-alignment-review.md) registra a conclusão da revisão histórica na baseline `fdc804f`. O plano [0.0.1](../0.0.1-plan.md) registra a qualificação técnica de G0–G5 com PostgreSQL 16 e fixtures sintéticas. Não se exige corpus de produção para concluir o software. Evidência externa continua pendente somente para ativar cada canal/finalidade; homologação de registro cadastral e as famílias G6/G7 permanecem expansão posterior.

## Prontidão

| Issue | Marco | Status | Condição de execução |
| --- | --- | --- | --- |
| [01 — Viabilidade de fontes](01-source-viability.md) | G0 | `needs-info` | Evidência de custo/uso por fonte e finalidade continua necessária antes de ativar dados reais; não bloqueia o software 0.0.1 |
| [02 — Domínio e contratos](02-domain-and-contracts.md) | G1 | `completed` | Contrato inicial, modelo e migrações qualificados; BVBG.028/TSL explicitamente adiadas até suportarem cortes por data interna |
| [03 — Aquisição e artefatos](03-artifact-acquisition.md) | G2 | `completed` | Aquisição controlada, raw/inventário/replay e limites validados com fixtures; ativação de canal real segue issue 01 |
| [04 — Núcleo histórico](04-historical-core.md) | G3 | `completed` | Cortes, revisão agregada, publicação automática limpa, retirada/reimport/rollback qualificados no PostgreSQL 16 sintético |
| [05 — Produto de consulta](05-query-product.md) | G4 | `completed` | REST/OpenAPI e MCP read-only qualificados para as capacidades disponíveis; dados reais dependem da issue 01 |
| [06 — Operação e recuperação](06-operations.md) | G5 | `completed` | Jobs, discovery, recuperação, build/smoke e benchmark/restore sintéticos qualificados; budgets de implantação ficam para ativação |
| [07 — Boletins de preços](07-price-reports.md) | G6 | `needs-info` | Plataforma 02–06 e corpus/elegibilidade próprios em 01 |
| [08 — Índices e carteiras](08-indices-and-portfolios.md) | G7 | `needs-info` | Plataforma 02–06 e corpus/elegibilidade próprios em 01 |
| [09 — Revisão de aderência](09-prd-alignment-review.md) | R0–R6 | `completed` | Conclusão registrada em 06/11/2026: baseline OpenAPI 3.1, matriz de aderência e plano de migração; não encerra os marcos de produto |

`needs-info` identifica evidência externa ainda não demonstrada e não afirma que o código correspondente esteja ausente. Fixtures sintéticas qualificam comportamento do software, mas não substituem autorização para adquirir, reter, processar ou distribuir dados reais.

## Ordem de trabalho

1. Manter os resultados da issue 09 como baseline histórica; evidências de 0.0.1 ficam no plano de lançamento e marcos correntes.
2. Resolver G0 para cada finalidade antes de ativar qualquer fonte real ou entregar seus dados externamente.
3. Homologar adaptadores/cobertura com publicações permitidas quando um canal for ativado; isso não reabre o aceite da plataforma.
4. Definir budgets de implantação quando houver hardware-alvo e corpus representativo autorizado.
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
- 06/11/2026 — Estado intermediário: implementação 0.0.1 com 65 testes passando em PostgreSQL 16.15 isolado; naquele momento o aceite externo ainda era tratado junto do release. A decisão posterior do mantenedor separou qualificação da plataforma e ativação por canal, registrada na entrada seguinte.
- 06/11/2026 — Decisão do mantenedor: release 0.0.1 qualifica a plataforma, não um corpus pré-carregado. `02`–`06` concluídas para o escopo 0.0.1 com 66 testes em PostgreSQL 16.15, migration upgrade, build/smoke, benchmark e restore documentados. `01` segue `needs-info` exclusivamente para ativação externa por canal/finalidade; G6/G7 seguem fora do primeiro release.
