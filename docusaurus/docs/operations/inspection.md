---
title: Inspeção de arquivos, members e falhas
---

# Inspeção operacional

Os endpoints de inspeção exigem credencial administrativa e retornam estado persistido. Eles são somente leitura: a consulta não extrai arquivos, refaz parsing nem altera uma revisão.

## Arquivos

GET /api/v1/b3/source-files lista artefatos em ordem determinística. Os itens informam ID, tipo de arquivo, canal, SHA-256, tamanho, versão do parser, estado e contadores de registros lidos, inseridos, alterados, inalterados, duplicados e em quarentena.

URLs tokenizadas e caminhos privados são omitidos. Os contadores refletem lotes efetivamente confirmados, não valores esperados ou planejados.

## Members

GET /api/v1/b3/source-files/&#123;id&#125;/members retorna entries originais do inventário, inclusive excluídas, com member_index, parent_index quando houver container aninhado, nome lógico, hash, tamanho, selected, exclusion_reason, estado, contadores e proveniência.

Entradas de mesmo nome dentro de containers diferentes são distintas. parent_index estabelece a relação entre member e arquivo contêiner.

## Rejeições

GET /api/v1/b3/data-revisions/&#123;id&#125;/quarantine retorna source_member_id, record_index, byte_offset quando conhecido, reason_code, mensagem controlada e referência da revisão/arquivo. O payload bruto não é devolvido nesta inspeção comum.

Replay é outro comando administrativo, rastreado como job. A rejeição original e sua evidência operacional não devem ser apagadas por uma tentativa posterior.

## Execuções

GET /api/v1/b3/sync-runs e GET /api/v1/b3/sync-runs/&#123;id&#125; são projeções tipadas de jobs duráveis. Use phase, heartbeat, tentativa e outputs para diferenciar fila, trabalho ativo, recurso produzido e falha terminal.
