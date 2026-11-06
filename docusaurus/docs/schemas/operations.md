---
title: Schemas operacionais
---

# Schemas e estados operacionais

## Submissão de job

POST /api/v1/b3/jobs aceita objeto fechado com:

| Campo | Tipo | Regras |
| --- | --- | --- |
| idempotency_key | string | Obrigatória, não vazia, até 255 caracteres |
| command | união discriminada | Um dos comandos administrativos fechados |
| priority | enum | maintenance, backfill ou rebuild |
| max_attempts | inteiro | Default 3, mínimo 1, máximo 100 |

Os comandos têm discriminador operation e não aceitam propriedades extras:

- ingest: source, from, to;
- reparse: source_file_id;
- replay_quarantine: source_file_id;
- rebuild_identity: from, to;
- publish_revision: revision_id, acknowledge_issues;
- rollback_revision: revision_id.

## JobView

Campos do job: id, retry_of, command, priority, status, phase, attempt, max_attempts, generation, heartbeat_at, cancel_requested_at, next_attempt_at, error_code, error_message, created_at, finished_at e outputs. Timestamps são UTC; outputs identificam recursos persistidos e não caminhos do host.

Status descreve execução: queued, running, succeeded, succeeded_with_issues, failed, cancelled. Phase descreve o estágio corrente: queued, acquisition, inventory, parsing, staging, sealing, ready, publication, identity ou completed.

## Revisão, conflito e quarentena

RevisionView contém family, id, base_revision_id, period_start, period_end, status, expected_members, rejected, conflicts, created_at, published_at e provenance. family separa cotahist de registry; status pode ser draft, ready, published ou invalid.

ConflictView localiza o fato e as duas versões conflitantes, member, índice e proveniência. RejectionView informa source_member_id, record_index, byte_offset quando disponível, reason_code, mensagem controlada e proveniência. A resposta comum não devolve arbitrariamente o payload bruto.

## Arquivo e member

SourceFileView informa id, tipo, canal, hash, tamanho, versão do parser, estado, quantidade de members e contadores de linhas lidas, inseridas, alteradas, inalteradas, duplicadas e em quarentena. MemberView informa nome lógico, índice determinístico, container pai, hash do conteúdo descomprimido, tamanho, seleção/exclusão, estado, contadores e provenance. Nenhum schema público expõe URL com token ou caminho privado.
