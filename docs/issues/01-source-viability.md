# 01 — Comprovar elegibilidade de fontes para ativação

Status: needs-info

Data: 05/11/2026. Marco: G0. Dependências: nenhuma.

## Problema

Disponibilidade pública, coleta automatizada, persistência e redistribuição possuem condições diferentes. O projeto não pode assumir que uma API para terceiros mantém custo zero de licença sem verificar essas condições.

## Escopo

- Registrar fonte/canal/finalidade, condições, versões e custo de aquisição/uso.
- Preparar perguntas à B3 sobre COTAHIST, cadastros, boletins, REST/MCP e distribuição de resultados, sem enviar mensagens externas sem autorização.
- Confirmar formas de acesso permitidas e horizonte declarado para cada canal/finalidade.
- Obter fixture/publicação delimitada para homologação quando permitido e útil; não é requisito para concluir o software 0.0.1.
- Medir colisões de chave e identificar schemas aplicáveis antes de habilitar adaptadores dependentes, sem varredura indiscriminada.

## Critérios de aceite

- Cada capacidade habilitada possui evidência suficiente de elegibilidade para a finalidade escolhida e custo compatível.
- Casos inconclusivos permanecem identificados; uma resposta genérica “dados públicos” não é encerramento.
- Quando um corpus for usado, origem, hash, data, versão e condições de retenção/uso ficam registrados.
- Horizonte real observado é separado do anúncio editorial e da cobertura desejada; corpus real não é pré-requisito de software.
- Se API externa gratuita não for viável, registrar a restrição e alternativas compatíveis, sem enfraquecer silenciosamente os requisitos.

## Fora do escopo

Bypass de restrições, contratação paga, migração de runtime e implementação de novos coletores antes da elegibilidade.

## Prontidão para execução

Informações externas de elegibilidade continuam pendentes. A plataforma 0.0.1 foi qualificada sem canal real habilitado; aquisição/distribuição de dados reais depende de evidência suficiente para a combinação específica de fonte e finalidade. A conclusão de software não depende de corpus oficial congelado.

## Comments

- 06/11/2026 — [Registro de elegibilidade](../release_source_eligibility.md) iniciado com consulta aos termos oficiais. Finalidade externa gratuita permanece não comprovada; nenhum corpus oficial adquirido.

- 05/11/2026 — Criada a partir da [pesquisa](../RESEARCH.md) e da seção 4 do [PRD](../b3_prd.md). Requer evidência externa; os demais trabalhos offline independentes podem avançar conforme o PRD vigente.

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
- 06/11/2026 — O mantenedor confirmou que ainda não há evidência de uso gratuito/redistribuição e instruiu manter o lançamento externo bloqueado. Nenhuma fonte ou corpus foi habilitado por esta decisão.
- 06/11/2026 — O mantenedor esclareceu que corpus (incluindo 2025–2026) serve à validação e não é critério de conclusão da plataforma; import e reimport idempotente são capacidades do produto. Esta issue segue aberta somente para elegibilidade e ativação por canal/finalidade.
