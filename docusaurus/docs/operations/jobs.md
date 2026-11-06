---
title: Jobs, retries e cancelamento
---

# Jobs administrativos

Jobs registram intenções de aquisição, reparse, replay, reconstrução de identidade e publicação. A API persiste um comando fechado; worker e CLI usam o mesmo estado durável.

## Enfileirar

POST /api/v1/b3/jobs recebe idempotency_key, command, priority e max_attempts opcional. O corpo rejeita campos desconhecidos. A resposta 202 contém job_id e status_url e também fornece cabeçalho Location.

Um comando ingest informa source, from e to. Reparse e replay_quarantine informam source_file_id. Rebuild_identity delimita o intervalo. publish_revision leva revision_id e acknowledge_issues. rollback_revision leva revision_id. O PRD e OpenAPI descrevem os enums e regras exatas aceitas.

A mesma chave com o mesmo comando efetivo retorna o mesmo job. Chave reutilizada com comando incompatível ou escopo concorrente retorna 409. Uma submissão não confirma conclusão.

## Acompanhar estado

GET /api/v1/b3/jobs/&#123;id&#125; requer administração. Observe status, phase, attempt/max_attempts, heartbeat_at, cancel_requested_at, error_code, error_message, created_at, finished_at e outputs. Mensagens de falha são sanitizadas.

Status terminal indica resultado; phase indica etapa de execução. Succeeded_with_issues não é sinônimo de publicação sem ressalvas.

## Cancelar

POST /api/v1/b3/jobs/&#123;id&#125;/cancel registra a solicitação e retorna 202. O worker verifica o pedido em pontos de controle seguros. Um commit que já venceu a corrida pode terminar; confirmações anteriores, raw e auditoria são preservados. Job já terminal retorna conflito.

## Retry

POST /api/v1/b3/jobs/&#123;id&#125;/retry aceita uma nova idempotency_key e só cria retry de execução falha ou cancelada. O comando, prioridade e limite original são preservados; a nova execução registra retry_of. A tentativa anterior continua no histórico. Reutilize a nova chave para obter o mesmo retry e evite repetir com uma chave diferente.

Uma execução iniciada pelo cliente é acompanhada por GET /api/v1/b3/jobs/&#123;id&#125;; o status HTTP da submissão não acompanha a conclusão do worker.
