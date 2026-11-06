# 04 — Entregar núcleo histórico COTAHIST e identidade disponível

Status: needs-triage

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

## Comments

- 05/11/2026 — Detalhamento nas seções 7–12 e 18 do [PRD](../../../docs/b3_prd.md). Não adicionar preços ajustados ou integração CVM.
