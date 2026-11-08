---
title: Endpoints COTAHIST
---

# Endpoints de cotações COTAHIST

As rotas de consulta usam Bearer de leitura, não iniciam aquisição e consultam uma revisão publicada elegível.

## GET /api/v1/b3/cotahist/quotes

Retorna fatos que correspondem aos filtros. Uma consulta pode incluir mais de uma linha por ticker e pregão quando os discriminadores forem diferentes.

| Parâmetro | Tipo | Default/limite | Significado |
| --- | --- | --- | --- |
| ticker_symbol | string | sem default | Ticker publicado; não é identidade permanente |
| instrument_id | UUID/texto | sem default | Instrumento canônico, quando associação temporal foi sustentada |
| isin | string | sem default | ISIN publicado na ocorrência |
| trade_date_from | data ISO | sem default | Primeiro pregão incluído |
| trade_date_to | data ISO | sem default | Último pregão incluído |
| market_type | string | sem default | Código de mercado, mantendo zeros à esquerda |
| bdi_code | string | sem default | Código BDI da ocorrência |
| distribution_number | string | sem default | Distribuição do papel/DISMES |
| term_days | string | sem default | Prazo a termo original |
| data_revision | UUID | revisão ativa por padrão | Fixa um conjunto publicado entre páginas |
| limit | inteiro | 100; 1–1000 | Máximo de registros por página |
| offset | inteiro | 0; mínimo 0 | Registros a ignorar |
| include_internal | booleano | false | Inclui metadados internos seguros |

Parâmetros inválidos e intervalos invertidos respondem 400. A resposta 200 inclui items, total, limit, offset, data_revision, disclaimer e coverage. Cada item contém identidade e proveniência. Uma fonte não elegível retorna 403; recurso/revisão inexistente, 404; revisão indisponível, 503.

Exemplo:

    GET /api/v1/b3/cotahist/quotes?ticker_symbol=PETR4&trade_date_from=2025-01-01&trade_date_to=2025-12-31&market_type=010&limit=100&offset=0

## GET /api/v1/b3/cotahist/quotes/export

Exporta todas as ocorrências COTAHIST de um ticker como um fluxo HTTP **finito** em `application/x-ndjson`. Não é feed de mercado em tempo real. Cada linha é um objeto JSON independente com o campo discriminador `type`; o fluxo termina somente quando o consumidor recebe `type: "complete"`.

| Parâmetro | Tipo | Default/limite | Significado |
| --- | --- | --- | --- |
| ticker_symbol | string | obrigatório; 1–64 caracteres | Ticker publicado, normalizado para maiúsculas; não identifica um instrumento permanente |
| data_revision | UUID | revisão ativa no início da requisição | Fixa a revisão imutável selecionada; copie-a do evento `meta` |
| cursor | string | sem default; máximo 1.024 caracteres | Cursor URL-safe opaco emitido em `checkpoint`; retoma depois de `(trade_date, id)` |
| include_internal | boolean | `false` | Inclui `internal.version_id` e `internal.business_hash` em cada cotação; não altera autorização |
| If-None-Match | header string | opcional | ETag de uma representação pronta; retorna `304` quando a representação selecionada não mudou |

Exemplo:

```http
GET /api/v1/b3/cotahist/quotes/export?ticker_symbol=PETR4
Authorization: Bearer <read-token>
Accept: application/x-ndjson
Accept-Encoding: gzip
```

O corpo usa estas linhas, nesta ordem geral:

1. `meta`: `schema_version`, `ticker_symbol`, `data_revision`, `resumed_after` e `disclaimer`.
2. Zero ou mais `quote`: `data` segue o schema `HistoricalQuote`, incluindo `identity` e `provenance`.
3. `checkpoint`: cursor exclusivo para continuar a partir do último fato emitido. É produzido após cada lote ordenado por data e UUID.
4. `complete`: `record_count` desta resposta, `last_cursor` e SHA-256 das linhas NDJSON anteriores. Sem este evento, trate a resposta como incompleta.

O cursor codifica ticker, revisão, versão do schema e a última chave `(trade_date, id)`. Para retomar, envie o mesmo ticker e cursor; a revisão é recuperada do cursor quando `data_revision` é omitida. Um UUID de revisão fornecido junto do cursor deve coincidir. Um cursor válido não promete que a revisão esteja mais elegível: REST e `third_party_delivery` são revalidados em cada chamada, inclusive em cache hits.

O endpoint não aceita `limit` ou `offset`: transmite o universo integral do ticker na revisão escolhida e ordena por `trade_date`, depois por UUID único. O banco fornece lotes keyset limitados; uma cache derivada pode ser reconstruída e tem validade de 30 dias. O armazenamento de desenvolvimento fica em `B3_PROCESSED_DATA_DIR/quote-exports`; instâncias que compartilham a mesma cache precisam compartilhar esse filesystem. O PostgreSQL e suas revisões permanecem a fonte de verdade.

Respostas são privadas (`Cache-Control: private`) e variam por `Authorization` e `Accept-Encoding`; não encaminhe a resposta a cache/CDN público. A primeira geração pode ser entregue sem compressão; artefatos prontos podem usar gzip. `ETag` e `Content-Length` aparecem somente para artefatos prontos. `If-None-Match` pode retornar `304` sem corpo.

Antes do início do corpo, erros usam o envelope HTTP comum: `400` filtro/cursor inválido, `401` Bearer inválido, `403` inelegibilidade, `404` ticker sem fatos, `500` falha interna ou `503` dependência/storage indisponível. Depois do início do fluxo, consulte eventos `error` e exija `complete`; erros de transporte também deixam o corpo incompleto. Um `200` que não contém `complete` não deve ser tratado como exportação bem-sucedida.

## GET /api/v1/b3/cotahist/series

Usa os mesmos filtros e envelope da consulta de fatos, mas exige uma seleção de série sem múltiplas linhas incompatíveis por pregão. Se mercado, distribuição ou prazo não desambiguarem a série, retorna 409 AMBIGUOUS_SERIES. A rota não faz forward-fill e não converte PREULT em preço genérico de fechamento.

## Interpretação obrigatória

A API publica valores nominais brutos na moeda e base de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos. A resposta conserva o aviso, FATCOT, unidade, proveniência e a situação de identidade. Consulte [schema COTAHIST](../schemas/cotahist.md) antes de construir séries financeiras.
