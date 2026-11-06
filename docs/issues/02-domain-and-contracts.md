# 02 — Fixar granularidade, identidade temporal e migração de contratos

Status: needs-info

Data: 05/11/2026. Marco: G1. Dependências: resultados de corpus de 01 para decisões definitivas.

## Problema

Ticker, instrumento, distribuição, fotografia de cadastro e revisão de arquivo não podem compartilhar identidade implícita. Chaves e respostas atuais precisam de inventário antes da evolução.

## Escopo

- Inventariar rotas efetivas, DTOs, enums, OpenAPI, chaves e consumidores conhecidos.
- Homologar qualificadores de security_id e relação entre BVBG.028, Tradable Security List e InstrumentsConsolidated.
- Especificar janelas, lacunas, chegada fora de ordem, ambiguidade e resolução tardia.
- Separar ocorrência, fato, revisão e política de seleção; especificar colisões e remoções.
- Registrar decisões aceitas em contextos/ADRs B3 quando justificadas e planejar novas migrations.

## Critérios de aceite

- Cenários de reuso de ticker, distribuições e prazo de termo têm resultados inequívocos ou incerteza explícita.
- Nenhuma chave de fato depende do vínculo reconstruível de instrumento.
- Cada alteração incompatível possui plano de versão/transição, incluindo novos estados e campos de cobertura.
- Históricos anteriores não recebem validade, publicação ou proveniência inventadas.
- Migration inicial permanece intacta; upgrade e inicialização vazia são previstos.

## Prontidão para execução

Concluir a revisão de aderência da issue 09 e obter os resultados aplicáveis da issue 01 para homologar chaves e relações. Inventário offline e hipóteses de projeto podem avançar antes do corpus, sem serem apresentados como homologação. O status deve mudar para `ready-for-agent` somente após registrar a evidência que libera a entrega completa.

## Comments

- 06/11/2026 — Execução 0.0.1 iniciada: fundação de elegibilidade/coordenação em migration aditiva. O mantenedor autorizou persistência nova e dispensou compatibilidade pré-lançamento. Modelo histórico e corpus continuam pendentes; ver [plano](../0.0.1-plan.md).

- 05/11/2026 — Requisitos em [PRD, seções 7–9, 12–13 e 19](../b3_prd.md). O [glossário](../domain/CONTEXT.md) acompanha o PRD vigente.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Revisão de aderência da issue 09 concluída com sucesso. Baseline de contratos e rotas formalizado em `.scratch/prd-alignment/evidence/contracts_inventory.md` e `.scratch/prd-alignment/evidence/openapi_reference.json`. Resta a conclusão da issue 01 para homologação do corpus.
