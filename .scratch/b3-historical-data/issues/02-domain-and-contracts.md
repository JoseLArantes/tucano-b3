# 02 — Fixar granularidade, identidade temporal e migração de contratos

Status: needs-triage

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

## Comments

- 05/11/2026 — Requisitos em [PRD, seções 7–9, 12–13 e 19](../../../docs/b3_prd.md). O [glossário](../CONTEXT.md) acompanha o PRD vigente.
