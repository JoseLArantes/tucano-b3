# Consumo do histórico nominal 0.0.1

Status: contrato 0.0.1 qualificado com fixtures sintéticas. Consultar o OpenAPI gerado em `/openapi.json` para o conjunto exato de operações registrado pelo binário. A plataforma não depende de dados reais pré-carregados; acesso a cada fonte continua dependente de elegibilidade própria.

Para a referência narrativa de endpoints, parâmetros, schemas e fluxos operacionais, consulte o [site Docusaurus](../docusaurus/docs/intro.md). O JSON em /openapi.json é a fonte de contrato legível por máquina; a rota /docs do serviço é uma página mínima com link para esse JSON, não o site Docusaurus.

Para um inventário detalhado do contrato HTTP implementado, fluxos de frontend, filtros, paginação, permissões, erros e relações entre recursos, consulte o [guia detalhado de consumo](consumers_guides.md).

## Autenticação e filtros

Enviar `Authorization: Bearer <credencial-de-leitura>` nas consultas protegidas. Credencial administrativa é separada e não deve ser distribuída a consumidores. `include_internal=true` não amplia permissões nem revela tokens/caminhos privados.

`GET /api/v1/b3/cotahist/quotes?ticker_symbol=EXMP3&market_type=010&trade_date_from=2025-01-01&trade_date_to=2025-12-31&limit=100&offset=0`

Ticker identifica um código no contexto e período. O mesmo ticker pode ter mercados, distribuições, prazos e instrumentos diferentes. Datas são inclusivas; paginação inválida retorna 400. `limit` padrão 100, máximo 1000; `offset` padrão 0. Filtros adicionais incluem instrumento, ISIN, BDI, distribuição e prazo.

## Precisão, revisão e cobertura

Valores financeiros são strings decimais. Preservar sua precisão; não converter para ponto flutuante para cálculos de auditoria. Moeda, fator de cotação e códigos originais continuam no registro. `last_price` corresponde ao último negócio publicado, sem garantir equivalência a outras medidas de fechamento.

Os preços são nominais brutos, sem ajuste retroativo por inflação ou eventos corporativos. Não há preenchimento de lacunas nem retorno total derivado apresentado como dado oficial.

Guardar `data_revision` da primeira página e enviá-lo nas páginas seguintes para fixar o conjunto. Ausência desse parâmetro seleciona a revisão ativa de cada requisição. `provenance` localiza arquivo, member e registro; `identity.status` mantém `resolved`, `unresolved` ou `ambiguous`. Vínculo ausente não invalida o preço preservado.

Cobertura não equivale a min/max de datas. Calendário desconhecido, rejeições, ausência de vínculo e limites do corpus permanecem explícitos. Uma dependência indisponível não é convertida em lista vazia.

`/cotahist/series` exige identificação do universo e retorna 409 quando várias linhas incompatíveis permanecem no mesmo dia. Acrescentar filtros de mercado, distribuição ou prazo até a seleção ser inequívoca.

## Evidências cadastrais

No contrato técnico, `/instruments` consulta fotografias publicadas por família. Porém, no `0.0.1`, BVBG.028 e Tradable Security List estão `deferred`, pois a data da fotografia não substitui o corte por data interna do registro. Essas rotas não representam uma fonte de identidade disponível neste release; consulte `/capabilities`.

Quando fotografia cadastral for habilitada em versão futura, fixe `snapshot_id` ao paginar. Não use um cadastro atual como mapeamento automático de todas as cotações antigas.

`GET /instruments/{id}`, `/instruments/{id}/identifiers` e `/tradable-securities` permanecem no schema, mas não retornam observações produzidas pelo fluxo suportado em `0.0.1`. Cotações sem vínculo cadastral continuam disponíveis com estado `unresolved`.

## Administração e MCP

Operações pesadas são comandos em `POST /jobs`; 202 significa intenção persistida, não conclusão. Acompanhar `status_url`, tentativas e outputs; cancelamento é solicitação durável e preserva confirmações anteriores. Arquivos, inventário, pendências, revisões, conflitos e quarentena têm inspeção administrativa própria.

## Importação e retirada administrativa

Somente um administrador inicia ingestão de uma família/canal oficial já configurado. Não existe upload de arquivos por consumidores. Importação local é restrita a operação controlada de artefatos oficiais, com origem e proveniência preservadas.

Todo comando `ingest` requer `from` e `to`, inclusivos. No `0.0.1`, só as granularidades COTAHIST são aceitas: o worker decide quais detalhes pertencem ao import pela `DATA DO PREGÃO` dentro de cada registro, ainda que adquira um arquivo anual. Nome, ano, header ou data de geração do arquivo só ajudam a localizar/identificar a publicação. Importar o ano inteiro exige enviar o intervalo completo explicitamente. Uma seleção que exige vários arquivos COTAHIST é consolidada numa revisão. BVBG.028/TSL são rejeitadas até que seu corte possa se basear numa data interna adequada de cada registro.

Um import limpo publica automaticamente depois de validações estruturais e dos gates de rejeição/conflito/elegibilidade. Se encontrar registros rejeitados ou conflito sem precedência, mantém a candidata fora da visão ativa e termina `succeeded_with_issues`. `withdraw_window` retira logicamente um intervalo da visão ativa numa nova revisão; raw, fatos/versões, auditoria e revisões anteriores não são apagados, permitindo rollback ou reimportação idempotente.

Elegibilidade externa é avaliada por fonte/canal/finalidade e pode deixar dados indisponíveis mesmo com os endpoints e parsers no software. Consulte [elegibilidade de fontes](release_source_eligibility.md) antes de habilitar um canal real.

MCP usa stdio. O cliente realiza initialize, envia notifications/initialized e consulta tools/list. As ferramentas usam os mesmos serviços de REST, com identidade, revisão, precisão e proveniência equivalentes. Não há sync, replay, rebuild, SQL arbitrário ou transporte HTTP MCP.
