# 04 — Entregar núcleo histórico COTAHIST e identidade disponível

Status: completed

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

PostgreSQL 16 real para NULLs, transações, concorrência e auditoria; repetir comandos Compose exigidos. Aplicar migrations sequenciais em banco vazio; não importar nem comparar contra estado persistido da implementação anterior.

## Prontidão para execução

O núcleo COTAHIST foi qualificado com fixtures sintéticas no PostgreSQL 16: cortes inclusivos por data interna, revisão multi-artefato, gates de qualidade, publicação automática de import limpo, retirada, reimport idempotente e rollback. Corpus real e elegibilidade são necessários para operar um canal com dados reais, não para concluir o software. Fontes cadastrais estão adiadas neste primeiro contrato.

## Comments

- 05/11/2026 — Detalhamento nas seções 7–12 e 18 do [PRD](../b3_prd.md). Não adicionar preços ajustados ou integração CVM.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Estado de uma iteração anterior ao contrato de auto-publicação: testes streaming cobriam largura/encoding/precisão, falhas e rebuild; dois workers validavam claim/recovery e uma fixture tipada exercitava TSL. O estado atual, com COTAHIST auto-publicado após gates e cadastro adiado, está registrado na atualização seguinte.
- 06/11/2026 — Qualificação integrada concluída no escopo COTAHIST sintético; import limpo publica automaticamente e fluxo de retirada/reimport/rollback passou. Os dados da fixture não são publicação B3 nem critério de cobertura de produção.
