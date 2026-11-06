---
title: Jobs, retries e cancelamento
---

# Jobs administrativos

Jobs registram intenções administrativas de aquisição, reparse, replay, reconstrução de identidade, retirada e rollback. A API persiste um comando fechado; worker e CLI usam o mesmo estado durável. Consumidores não enviam arquivos nem iniciam ingestão.

## Enfileirar

POST /api/v1/b3/jobs recebe idempotency_key, command, priority e max_attempts opcional. O corpo rejeita campos desconhecidos. A resposta 202 contém job_id e status_url e também fornece cabeçalho Location.

Um comando ingest informa uma família oficial em source, from e to. No `0.0.1`, só COTAHIST anual, mensal ou diário é aceito: as datas inclusivas cortam `DATA DO PREGÃO` de cada detalhe. Metadados do arquivo servem para localizar a publicação, não para ampliar os fatos importados. Um ano inteiro requer `2025-01-01` a `2025-12-31` de forma explícita. Se várias publicações COTAHIST forem necessárias, elas formam uma única revisão candidata. BVBG.028 e Tradable Security List são reportadas como `deferred` em `/capabilities` e a submissão é rejeitada até que haja corte confiável sobre uma data interna por registro.

```json
{
  "idempotency_key": "cotahist-2025-01-02",
  "command": {
    "operation": "ingest",
    "source": "COTAHIST_YEARLY",
    "from": "2025-01-02",
    "to": "2025-01-02"
  },
  "priority": "backfill"
}
```

`reparse` e `replay_quarantine` informam `source_file_id`; `rebuild_identity` delimita intervalo; `publish_revision` leva `revision_id` e `acknowledge_issues`; `rollback_revision` leva `revision_id`; `withdraw_window` recebe `from` e `to` inclusivos e retira datas da visão ativa sem apagar o raw ou as versões anteriores.

A mesma chave com o mesmo comando efetivo retorna o mesmo job. Chave reutilizada com comando incompatível ou escopo concorrente retorna 409. Uma submissão não confirma conclusão.

## Acompanhar estado

GET /api/v1/b3/jobs/&#123;id&#125; requer administração. Observe status, phase, attempt/max_attempts, heartbeat_at, cancel_requested_at, error_code, error_message, created_at, finished_at e outputs. Mensagens de falha são sanitizadas.

Status terminal indica resultado; phase indica etapa de execução. Um import estruturalmente íntegro, sem rejeições/conflitos e com fonte elegível publica automaticamente. `succeeded_with_issues` indica que existe candidato não publicado aguardando decisão administrativa. Falha estrutural ou de infraestrutura não é sucesso degradado.

## Cancelar

POST /api/v1/b3/jobs/&#123;id&#125;/cancel registra a solicitação e retorna 202. O worker verifica o pedido em pontos de controle seguros. Um commit que já venceu a corrida pode terminar; confirmações anteriores, raw e auditoria são preservados. Job já terminal retorna conflito.

## Retry

POST /api/v1/b3/jobs/&#123;id&#125;/retry aceita uma nova idempotency_key e só cria retry de execução falha ou cancelada. O comando, prioridade e limite original são preservados; a nova execução registra retry_of. A tentativa anterior continua no histórico. Reutilize a nova chave para obter o mesmo retry e evite repetir com uma chave diferente.

Uma execução iniciada pelo cliente é acompanhada por GET /api/v1/b3/jobs/&#123;id&#125;; o status HTTP da submissão não acompanha a conclusão do worker.
