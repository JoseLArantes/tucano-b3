# 06 — Operar descoberta, fila e manutenção com recuperação

Status: completed

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

## Prontidão para execução

Jobs, descoberta e recuperação foram qualificados para o software 0.0.1 com testes PostgreSQL, build e smoke API/worker, benchmark e restore com consulta REST. O benchmark sintético não define budgets de produção. Calendário e descoberta de um canal real exigem evidência oficial e elegibilidade específica; sem isso, a cobertura permanece desconhecida e o canal desligado.

## Comments

- 06/11/2026 — Primeiro estágio: serviço interno de jobs duráveis implementado para W3: idempotência, lanes, lease/heartbeat, fencing, tentativas e cancelamento. Os itens listados como pendentes foram implementados depois; consultar a atualização abaixo.

- 05/11/2026 — Seções 10–11, 14 e 17–18 do [PRD](../b3_prd.md). Este marco começa antes da conclusão do núcleo histórico; recuperação não é fase opcional posterior.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Jobs integrados a API/CLI/worker, ingestão e publicação passam por fencing; testes PostgreSQL cobrem retry, cancelamento, lease expirada e dois processos reais. Discovery registra pendências sem ingerir, usa evidência de calendário e preserva `unknown`. Build/smoke e benchmark/restauração com 20.000 fatos sintéticos passaram e estão registrados no guia operacional; budgets de produção aguardam corpus autorizado/hardware-alvo.
- 06/11/2026 — Fechado para o aceite de plataforma 0.0.1. A definição de budgets de implantação, operação com fonte real e calendário versionado segue como trabalho por canal após elegibilidade, sem reabrir G5 do software.
