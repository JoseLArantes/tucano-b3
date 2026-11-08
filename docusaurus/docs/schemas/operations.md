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
- rollback_revision: revision_id;
- withdraw_window: from, to inclusivos, sem remoção física.

`ingest.from`/`to` são cortes inclusivos na data de negócio dos registros. No `0.0.1`, somente COTAHIST é aceito e usa `DATA DO PREGÃO`; metadados de publicação ajudam a selecionar artefatos, mas não substituem esse filtro. BVBG.028 e Tradable Security List são rejeitadas até que os registros tenham uma referência interna adequada. Upload de arquivo por consumidor não é suportado.

## JobView

Campos do job: id, retry_of, command, priority, status, phase, attempt, max_attempts, generation, heartbeat_at, cancel_requested_at, next_attempt_at, error_code, error_message, created_at, finished_at e outputs. Timestamps são UTC; outputs identificam recursos persistidos e não caminhos do host.

Status descreve execução: queued, running, succeeded, succeeded_with_issues, failed, cancelled. Phase descreve o estágio corrente: queued, acquisition, inventory, parsing, staging, sealing, ready, publication, identity ou completed.

## Revisão, conflito e quarentena

RevisionView contém family, id, revision_kind, source_file_ids, base_revision_id, period_start, period_end, status, expected_members, rejected, out_of_scope, conflicts, created_at, published_at e provenance. `family` separa cotahist de registry; `revision_kind` é ingest, identity, withdrawal ou registry_snapshot; status pode ser draft, ready, published ou invalid. Para COTAHIST, o intervalo é o corte solicitado e inclusivo. `out_of_scope` conta detalhes válidos no artefato que não pertencem ao corte.

Um import limpo é promovido automaticamente. Rejeições e conflitos mantêm a candidata fora da visão ativa até decisão administrativa. Uma revisão `withdrawal` omite as datas solicitadas, mas preserva a evidência histórica para rollback/reimportação.

ConflictView localiza o fato e as duas versões conflitantes, member, índice e proveniência. RejectionView informa source_member_id, record_index, byte_offset quando disponível, reason_code, mensagem controlada e proveniência. A resposta comum não devolve arbitrariamente o payload bruto.

## Arquivo e member

SourceFileView informa id, tipo, canal, hash, tamanho, versão do parser, estado, quantidade de members e contadores de linhas lidas, inseridas, alteradas, inalteradas, duplicadas e em quarentena. MemberView informa nome lógico, índice determinístico, container pai, hash do conteúdo descomprimido, tamanho, seleção/exclusão, estado, contadores e provenance. Nenhum schema público expõe URL com token ou caminho privado.

## DiscoveryResult

Payload do `POST /api/v1/b3/discovery/scan`. A operação registra a checagem e retorna candidatos encontrados nas pastas, URLs B3 ou buckets S3 configurados. O scanner não obtém corpo nem prova elegibilidade ou publicação.

| Campo | Tipo/formato | Interpretação |
|---|---|---|
| `checked_at` | timestamp RFC 3339 UTC | Instante de referência da chamada manual. |
| `scheduled` | boolean | `false` para a chamada manual; não renova saúde do worker. |
| `locations_scanned` | inteiro | Número de locais configurados processados. |
| `candidate_count` | inteiro | Total de arquivos candidatos observados dentro dos limites por local. |
| `candidates_truncated` | boolean | `true` quando a lista excede 1.000 itens ou algum local atingiu limite. |
| `locations` | lista fechada de `DiscoveryLocationResult` | Resultado individual, incluindo limites e falhas. |
| `candidates` | lista fechada de `DiscoveryCandidateResult`, até 1.000 itens | Metadados dos arquivos COTAHIST detectados. |

`DiscoveryLocationResult.kind` é `folder`, `b3_url` ou `object_storage_bucket`; `label` identifica a posição na configuração (por exemplo `folder-1`) sem expor a raiz privada; `outcome` é `available`, `absent`, `inconclusive` ou `failed`; `candidate_count` e `truncated` descrevem a busca naquele local. Uma falha individual não interrompe os demais locais.

`DiscoveryCandidateResult.source_file_type` é `COTAHIST_YEARLY`, `COTAHIST_MONTHLY` ou `COTAHIST_DAILY`, inferido pelo nome; `source_locator` aponta para URL B3, URI S3 ou caminho relativo prefixado por `folder:<índice>/`; `byte_size` é o tamanho declarado em bytes; `remote_etag` e `remote_last_modified` são opcionais. ETag não é checksum. A descoberta não confirma que os bytes correspondem ao nome e não infere data de referência, integridade, precedência ou autorização de redistribuição.

`DiscoveryFileType` é um enum fechado com `COTAHIST_YEARLY`, `COTAHIST_MONTHLY` e `COTAHIST_DAILY`; nenhum outro tipo de fonte é retornado por esse endpoint.
