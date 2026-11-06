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

O pipeline técnico foi implementado com aquisição HTTPS/local, hashing incremental, raw imutável, inventário aninhado limitado e replay. O aceite completo permanece `needs-info` até obter elegibilidade da issue 01 e homologar os canais/fontes reais; nenhuma aquisição B3 foi executada nesta entrega.

## Comments

- 05/11/2026 — Requisitos nas seções 10, 11 e 18 do [PRD](../b3_prd.md). Coordenar cancelamento e fencing com a issue 06 desde o início.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — Testes sintéticos cobrem limites de bytes/membros/metadados, ZIP aninhado, path traversal, raw alterado, URL não pública e resposta HTTP simulada. Homologação de arquivo/canal oficial e medições de corpus permanecem pendentes.
