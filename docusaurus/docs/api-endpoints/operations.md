---
title: Endpoints operacionais
---

# Endpoints operacionais e administrativos

Operações usam os mesmos jobs e estados PostgreSQL compartilhados entre API, CLI e worker. GET de inspeção operacional e todos os métodos POST exigem Bearer administrativo, com exceção das rotas explicitamente identificadas como leitura geral.

## Jobs

| Método e rota | Acesso | Efeito |
| --- | --- | --- |
| GET /api/v1/b3/jobs | Admin | Lista jobs por criação e UUID |
| POST /api/v1/b3/jobs | Admin | Persiste comando fechado e devolve 202, job_id e status_url |
| GET /api/v1/b3/jobs/&#123;id&#125; | Admin | Lê estado, fase, tentativas, heartbeat, falha segura e outputs |
| POST /api/v1/b3/jobs/&#123;id&#125;/cancel | Admin | Registra solicitação durável de cancelamento |
| POST /api/v1/b3/jobs/&#123;id&#125;/retry | Admin | Cria retry vinculado a job falho/cancelado, preservando histórico |
| GET /api/v1/b3/sync-runs | Admin | Lista execuções refletidas no registro operacional |
| GET /api/v1/b3/sync-runs/&#123;id&#125; | Admin | Consulta a execução durável por identificador |

POST /jobs recebe idempotency_key, command, priority e max_attempts opcional. max_attempts padrão 3, faixa de 1 a 100. Commands são ingest, reparse, replay_quarantine, rebuild_identity, publish_revision, rollback_revision ou withdraw_window. Reuso da chave com comando idêntico retorna o mesmo job; uso incompatível retorna 409. O POST não faz download ou parsing dentro da requisição.

`ingest` exige uma família B3 oficial configurada e `from`/`to` inclusivos sobre a data de negócio. No `0.0.1`, somente COTAHIST diário, mensal ou anual é aceito: o worker seleciona cada fato pela `DATA DO PREGÃO`; o período declarado do arquivo só localiza possíveis publicações. O administrador envia o ano inteiro explicitamente se esse for o corte desejado. Publicações COTAHIST múltiplas selecionadas no mesmo job são consolidadas em uma revisão atômica. BVBG.028 e Tradable Security List permanecem `deferred` e são rejeitadas no comando de ingestão.

Não há endpoint de upload. Importação local é configuração administrativa controlada para artefato oficial rastreável, não uma forma de consumidores enviarem dados. Os canais/finalidades sem elegibilidade permanecem desabilitados.

Um import limpo publica automaticamente após os gates estruturais, de rejeição, conflito e elegibilidade. Falhas de qualidade deixam a candidata não ativa e o job `succeeded_with_issues`. `withdraw_window` publica uma nova visão sem as datas inclusivas solicitadas; raw, fatos, versões, ocorrências e revisões anteriores não são apagados e rollback pode restaurar a visão anterior.

## Descoberta e pendências

| Método e rota | Acesso | Efeito |
| --- | --- | --- |
| POST /api/v1/b3/discovery/scan | Admin | Registra uma descoberta leve de publicações |
| GET /api/v1/b3/pending-updates | Admin | Lista publicações pendentes conhecidas |
| POST /api/v1/b3/pending-updates/&#123;id&#125;/acknowledge | Admin | Registra reconhecimento administrativo da pendência |

Descoberta não equivale a download ou ingestão. Acknowledgement também não inicia processamento.

## Arquivos e revisões

| Método e rota | Acesso | Efeito |
| --- | --- | --- |
| GET /api/v1/b3/source-files | Admin | Lista artefatos e contadores persistidos |
| GET /api/v1/b3/source-files/&#123;id&#125;/members | Admin | Lista inventário ordenado, entries selecionadas e excluídas |
| GET /api/v1/b3/data-revisions/&#123;id&#125; | Admin | Inspeciona estado e escopo da revisão/snapshot |
| GET /api/v1/b3/data-revisions/&#123;id&#125;/conflicts | Admin | Lista versões conflitantes sem escolher vencedora |
| GET /api/v1/b3/data-revisions/&#123;id&#125;/quarantine | Admin | Lista ocorrências rejeitadas com localização |

## Rotas de leitura geral

GET /api/v1/b3/capabilities lista capacidade cadastrada e estado de elegibilidade. GET /api/v1/b3/diagnostics/coverage agrega limites e identidade de uma revisão publicável. Essas rotas usam token Bearer de leitura, não precisam ser administrativas, e não disparam aquisição.

## Erros e estado

202 é aceitação, não conclusão. 409 indica conflito de chave/escopo, tentativa incompatível ou estado terminal não retriável. 403 indica credencial de leitura em operação administrativa. 404 identifica job, arquivo, member, pendência ou revisão ausente. Códigos de falha são sanitizados; não contêm SQL, tokens nem caminhos do host.
