# 06 — Operar descoberta, fila e manutenção com recuperação

Status: needs-triage

Data: 05/11/2026. Marco: G5. Dependências: 02; desenvolvimento conjunto com 03/04.

## Problema

Um loop que dispara ingestões a cada minuto e verifica finais de semana não demonstra manutenção confiável de dados B3 nem coordenação entre processos.

## Escopo

- Scanner leve e pendências com checagens conclusivas/inconclusivas e baseline reconhecido.
- Fila PostgreSQL com prioridade, claim, lease, fencing, heartbeat, retry e cancelamento durável.
- Locks de promoção por sobreposição real entre granularidades.
- Calendário B3 versionado e timezone explícito; política de execução administrativa separada da descoberta.
- API operacional por run/member/fase, métricas, limpeza idempotente, backup e restauração.

## Critérios de aceite

- Dois processos e um worker antigo com lease expirada não publicam resultados conflitantes.
- Backfill, rebuild e consultas não esgotam a capacidade de manutenção regular.
- Cancelamento funciona nas fases previstas e não depende de processo local único.
- Saúde agendada não é renovada artificialmente por execução manual.
- Ausência de calendário fica desconhecida; manutenção de dados antigos funciona em fim de semana.
- Operador identifica problema e recuperação pela API; restore e replay são ensaiados.

## Validação

PostgreSQL 16 real, comandos Compose obrigatórios, build e smoke de API/worker com aquisição externa controlada.

## Comments

- 05/11/2026 — Seções 10–11, 14 e 17–18 do [PRD](../PRD.md). Este marco começa antes da conclusão do núcleo histórico; recuperação não é fase opcional posterior.
