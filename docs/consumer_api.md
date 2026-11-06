# Consumo do histórico nominal 0.0.1

Status: contrato inicial implementado e coberto por testes/ smoke sintéticos. Consultar o OpenAPI gerado em `/openapi.json` para o conjunto exato de operações registrado pelo binário. A publicação externa depende dos gates de elegibilidade e qualificação, não somente da presença das rotas.

Para a referência narrativa de endpoints, parâmetros, schemas e fluxos operacionais, consulte o [site Docusaurus](../docusaurus/docs/intro.md). O JSON em /openapi.json é a fonte de contrato legível por máquina; a rota /docs do serviço é uma página mínima com link para esse JSON, não o site Docusaurus.

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

`/instruments` consulta fotografias publicadas por família. Cada item informa `snapshot_id`, referência, qualificador de `security_id` e proveniência. Duas fontes podem sustentar observações distintas do mesmo instrumento. A referência da fotografia não prova início de validade do identificador.

Fixar `snapshot_id` ao paginar uma fotografia específica. Não usar um cadastro atual como mapeamento automático de todas as cotações antigas.

`GET /instruments/{id}` retorna todas as evidências cadastrais atualmente publicadas para o instrumento; snapshots de famílias diferentes permanecem como itens distintos e não há seleção arbitrária do primeiro. `GET /instruments/{id}/identifiers` lista identificadores qualificados, as janelas explicitamente sustentadas e proveniência de cada observação. Uma fotografia com apenas `reference_date` gera uma observação nessa data, não uma validade contínua presumida. `GET /tradable-securities` restringe a consulta à família Tradable Security List, sem fallback a BVBG.028; campos ausentes na projeção continuam nulos.

## Administração e MCP

Operações pesadas são comandos em `POST /jobs`; 202 significa intenção persistida, não conclusão. Acompanhar `status_url`, tentativas e outputs; cancelamento é solicitação durável e preserva confirmações anteriores. Arquivos, inventário, pendências, revisões, conflitos e quarentena têm inspeção administrativa própria.

MCP usa stdio. O cliente realiza initialize, envia notifications/initialized e consulta tools/list. As ferramentas usam os mesmos serviços de REST, com identidade, revisão, precisão e proveniência equivalentes. Não há sync, replay, rebuild, SQL arbitrário ou transporte HTTP MCP.
