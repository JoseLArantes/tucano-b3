# Alterações da API para consumidores

## 06/11/2026 — Contrato inicial 0.0.1 implementado; publicação externa bloqueada

Primeiro contrato suportado, sem obrigação de compatibilidade com a implementação anterior ao lançamento. A implementação do contrato não libera fontes para terceiros: elegibilidade externa e corpus oficial continuam sendo gates separados.

- Namespace REST: `/api/v1/b3`; versão de software independente da versão do namespace.
- Consulta COTAHIST: `/cotahist/quotes` e `/cotahist/series`, com valores financeiros em strings, `data_revision`, cobertura, estado de identidade e proveniência até member/registro. Série ambígua retorna 409, sem escolher uma linha arbitrária.
- Cadastro: `/instruments` retorna evidências de fotografias explicitamente publicadas, com `snapshot_id`, namespace do código e origem. Não transforma fotografia atual em validade histórica.
- Cadastro detalhado: `/instruments/{id}` mantém as evidências de snapshots distintos em itens separados; `/instruments/{id}/identifiers` retorna valores, qualificadores, validade demonstrada e proveniência; `/tradable-securities` força a família TSL sem fallback a BVBG.028.
- Administração: `POST /jobs` retorna 202 com `job_id`, `status_url` e `Location`. `GET /jobs`, detalhe e cancelamento usam os mesmos estados duráveis da CLI e do worker. Nenhuma aquisição é executada durante consultas analíticas.
- Inspeção: arquivos/members, revisões/conflitos/quarentena, pendências e catálogo de capacidades. Famílias posteriores a G5 ficam explicitamente adiadas no catálogo.
- Autorização: credenciais Bearer distintas de leitura/administração; 401 para credencial inválida e 403 para leitura tentando administrar. Variável antiga `TUCANO_B3_TOKEN` não concede acesso ao contrato novo.
- Erros REST: `code`, `message`, `request_id`, sem SQL, credenciais ou caminhos privados.
- MCP inicial: stdio, ferramentas analíticas somente leitura, com os mesmos serviços de consulta; índices/carteiras não são expostos antes de sua homologação.

Compatibilidade: consumidores pré-lançamento devem adotar o contrato inicial e suas credenciais novas. Reprocessamento, republicação e rebuild posteriores ao lançamento devem preservar as garantias de identidade, versão e auditoria do PRD.
