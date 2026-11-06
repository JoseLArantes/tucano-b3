# 02 — Fixar granularidade, identidade temporal e contrato 0.0.1

Status: completed

Data: 05/11/2026. Marco: G1. Dependências: resultados de corpus de 01 para decisões definitivas.

## Problema

Ticker, instrumento, distribuição, fotografia de cadastro e revisão de arquivo não podem compartilhar identidade implícita. A implementação anterior serve como referência de código; `0.0.1` define o primeiro contrato suportado pelo PRD, sem migrar registros/IDs ou herdar obrigações de compatibilidade de respostas anteriores.

## Escopo

- Definir rotas, DTOs, enums, OpenAPI, chaves e filtros do primeiro contrato; usar o inventário anterior para identificar capacidades reutilizáveis e riscos, não como garantia de compatibilidade.
- Homologar qualificadores de security_id e relação entre BVBG.028, Tradable Security List e InstrumentsConsolidated.
- Especificar janelas, lacunas, chegada fora de ordem, ambiguidade e resolução tardia.
- Separar ocorrência, fato, revisão e política de seleção; especificar colisões e remoções.
- Registrar decisões aceitas em contextos/ADRs B3 quando justificadas e planejar novas migrations.

## Critérios de aceite

- Cenários de reuso de ticker, distribuições e prazo de termo têm resultados inequívocos ou incerteza explícita.
- Nenhuma chave de fato depende do vínculo reconstruível de instrumento.
- A interface inicial corresponde ao PRD; alterações depois de `0.0.1` seguem a política de compatibilidade publicada e incluem transição quando incompatíveis.
- Históricos anteriores não recebem validade, publicação ou proveniência inventadas.
- Migrations existentes não são reescritas; a sequência instala o schema em banco vazio sem importar registros, IDs, auditoria ou estado operacional da implementação anterior.

## Prontidão para execução

O modelo e o contrato inicial foram qualificados com migrations e fixtures no PostgreSQL 16. Não se exige corpus de produção para o software 0.0.1. A homologação de identidade de fontes cadastrais permanece fora deste escopo, pois BVBG.028/TSL estão `deferred` até suportarem cortes de data por registro.

## Comments

- 06/11/2026 — Execução 0.0.1 iniciada: fundação de elegibilidade/coordenação em migration aditiva. O mantenedor autorizou persistência nova e dispensou compatibilidade pré-lançamento. Modelo histórico e corpus continuam pendentes; ver [plano](../0.0.1-plan.md).

- 05/11/2026 — Requisitos em [PRD, seções 7–9, 12–13 e 19](../b3_prd.md). O [glossário](../domain/CONTEXT.md) acompanha o PRD vigente.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Revisão de aderência da issue 09 concluída com sucesso. Baseline de contratos e rotas formalizado em `.scratch/prd-alignment/evidence/contracts_inventory.md` e `.scratch/prd-alignment/evidence/openapi_reference.json`. Resta a conclusão da issue 01 para homologação do corpus.
- 06/11/2026 — Implementadas migrations aditivas `0004`–`0011`, fatos tipados/versionados, membership de revisão, snapshots de cadastro e rebuild de identidade. PostgreSQL testa correção, rejeição, conflito, rollback e rebuild sem alterar versões de mercado. Homologação de natural keys/qualificadores com corpus B3 autorizado continua pendente; ver [plano 0.0.1](../0.0.1-plan.md).
- 06/11/2026 — Fechado para o contrato de plataforma 0.0.1: migrations `0001`–`0012`, corte/importação, revisão e invariantes passaram na suíte PostgreSQL 16; a migration `0012` também foi aplicada como upgrade isolado de `0011`. Validação de formatos/identidades cadastrais é expansão futura e não bloqueia o primeiro release.
