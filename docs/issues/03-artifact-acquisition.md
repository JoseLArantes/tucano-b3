# 03 — Implementar aquisição limitada e inventário recuperável

Status: needs-info

Data: 05/11/2026. Marco: G2. Dependências: 02; 01 para aquisição de dados externos.

## Problema

Download e extração atuais materializam conteúdo em memória. Falha, limites e proveniência de ZIPs aninhados precisam de comportamento verificável.

## Escopo

- Adaptadores por canal permitido, download incremental, raw imutável e manifesto.
- Validação de HTTPS, redirects, host/destino de rede e fluxo com token quando homologado.
- Inventário completo e determinístico da árvore de members, com entradas repetidas e exclusões justificadas.
- Limites de bytes, expansão, quantidade, profundidade, XML e registro.
- Importação local confinada e procedência, sem caminho arbitrário recebido de consumidor.

## Critérios de aceite

- Mocks cobrem HTML 200, falhas HTTP, redirect indevido, interrupção e token expirado.
- Arquivo incompleto nunca ganha estado equivalente a aquisição válida.
- Nomes iguais em ZIPs diferentes têm localizadores distintos.
- Limite excedido é falha visível e não desaparecimento silencioso de conteúdo.
- Replay recupera bytes por manifesto, sem necessidade de novo acesso externo.
- Memória e disco temporário são medidos em corpus grande controlado.

## Validação

Comandos Compose obrigatórios do PRD; build e smoke API/worker se houver mudança de runtime. Nenhuma aquisição real automática nos testes.

## Prontidão para execução

Aguardar o contrato de artefatos/escopos da issue 02; acesso externo depende da issue 01. Mocks e desenho offline podem ser preparados sem habilitar coletores reais. O status deve mudar para `ready-for-agent` somente após registrar a evidência que libera a entrega completa.

## Comments

- 05/11/2026 — Requisitos nas seções 10, 11 e 18 do [PRD](../b3_prd.md). Coordenar cancelamento e fencing com a issue 06 desde o início.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
