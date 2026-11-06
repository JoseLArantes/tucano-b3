# 04 — Entregar núcleo histórico COTAHIST e identidade disponível

Status: needs-info

Data: 05/11/2026. Marco: G3. Dependências: 01, 02 e 03; coordenação com 06.

## Problema

O histórico precisa continuar útil sem cadastro antigo completo, preservando moeda, unidade, versões e recuperação transacional.

## Escopo

- Parser por bytes/encoding homologado, escalas, controles, distribuição, prazo, identificador histórico e isolamento de linhas inválidas.
- Cadastros disponíveis prioritários, com contratos distintos e resolução temporal sem retroatividade inventada.
- Lotes limitados, staging tipado quando necessário, comparação em conjunto e auditoria atômica.
- Revisão publicável por escopo, observações de suporte e política de conflito de granularidades.
- Reparse, replay, rebuild e remoção controlada de linhas em substituições completas.

## Critérios de aceite

- Sobreposição diário/mensal/anual equivalente não duplica cotação.
- Divergências sem precedência não são resolvidas pela ordem de processamento.
- Reingestão e rebuild preservam preços e timestamps de mudança quando não há alteração de negócio.
- Uma linha inválida é preservada; header/trailer inválido impede publicação completa.
- Falha após lotes confirmados mantém revisão anterior servível e permite retomada.
- Corpus delimitado tem mapa de cobertura e benchmark reproduzível.

## Validação

PostgreSQL 16 real para NULLs, transações, concorrência e auditoria; repetir comandos Compose exigidos. Cobrir upgrade e banco vazio.

## Prontidão para execução

O núcleo técnico foi implementado sobre as issues 02, 03 e 06, com publicação explícita e fixtures sintéticas no PostgreSQL. O aceite completo ainda depende do corpus elegível da issue 01, da reconciliação entre granulações e do benchmark com uma amostra permitida.

## Comments

- 05/11/2026 — Detalhamento nas seções 7–12 e 18 do [PRD](../b3_prd.md). Não adicionar preços ajustados ou integração CVM.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Testes streaming cobrem largura/encoding/precisão, isolamento de linhas inválidas, falhas estruturais, correção/conflito/rollback e rebuild cadastral. Dois workers de processo real validam claim/recovery/publicação explícita; uma fixture tipada valida TSL. Sem corpus oficial, isso não constitui homologação de produção.
