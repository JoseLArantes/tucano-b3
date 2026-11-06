# 01 — Comprovar viabilidade de fontes e obter corpus homologável

Status: needs-info

Data: 05/11/2026. Marco: G0. Dependências: nenhuma.

## Problema

Disponibilidade pública, coleta automatizada, persistência e redistribuição possuem condições diferentes. O projeto não pode assumir que uma API para terceiros mantém custo zero de licença sem verificar essas condições.

## Escopo

- Registrar fonte/canal/finalidade, condições, versões e custo de aquisição/uso.
- Preparar perguntas à B3 sobre COTAHIST, cadastros, boletins, REST/MCP e distribuição de resultados, sem enviar mensagens externas sem autorização.
- Confirmar formas de acesso permitidas e horizonte declarado de cada família.
- Obter corpus delimitado de forma permitida: períodos antigos/recentes, transições, arquivos sobrepostos e exemplos oficiais de identidade.
- Medir colisões de chave e identificar schemas aplicáveis, sem uma varredura indiscriminada.

## Critérios de aceite

- Cada capacidade habilitada possui evidência suficiente de elegibilidade para a finalidade escolhida e custo compatível.
- Casos inconclusivos permanecem identificados; uma resposta genérica “dados públicos” não é encerramento.
- Corpus tem origem, hash, data, versão e condições de retenção/uso em testes.
- Horizonte real observado é separado do anúncio editorial e da cobertura desejada.
- Se API externa gratuita não for viável, registrar a restrição e alternativas compatíveis, sem enfraquecer silenciosamente os requisitos.

## Fora do escopo

Bypass de restrições, contratação paga, migração de runtime e implementação de novos coletores antes da elegibilidade.

## Prontidão para execução

Informações externas de elegibilidade e corpus ainda pendentes. Preparação de perguntas e inventário podem avançar; aquisição real e conclusão dependem de evidência suficiente. O status deve mudar para `ready-for-agent` somente após registrar a evidência que libera a entrega completa.

## Comments

- 06/11/2026 — [Registro de elegibilidade](../release_source_eligibility.md) iniciado com consulta aos termos oficiais. Finalidade externa gratuita permanece não comprovada; nenhum corpus oficial adquirido.

- 05/11/2026 — Criada a partir da [pesquisa](../RESEARCH.md) e da seção 4 do [PRD](../b3_prd.md). Requer evidência externa; os demais trabalhos offline independentes podem avançar conforme o PRD vigente.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — O mantenedor confirmou que ainda não há evidência de uso gratuito/redistribuição e instruiu manter o lançamento externo bloqueado. Nenhuma fonte ou corpus foi habilitado por esta decisão.
