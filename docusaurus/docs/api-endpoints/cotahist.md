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

Exporta todas as ocorrências oficiais COTAHIST de um ticker em uma revisão publicada como fluxo HTTP **finito** `application/x-ndjson`. Não é feed de mercado em tempo real. Cada linha UTF-8 é um objeto JSON terminado por LF (`\n`) e possui o campo discriminador `type`. A conexão HTTP pode fragmentar uma linha em vários chunks ou entregar várias linhas no mesmo chunk; consumidores devem montar linhas por LF, sem tratar chunk de transporte como evento.

A revisão é escolhida uma vez no início (ou recuperada do cursor) e não muda se outra revisão for publicada durante a chamada. O endpoint retorna todas as ocorrências do ticker, incluindo mercados, BDI, distribuições e prazos distintos; preserve esses qualificadores do registro para evitar misturar universos diferentes. A ordem crescente é `trade_date`, depois UUID `id` como desempate único.

| Parâmetro | Tipo | Default/limite | Significado |
| --- | --- | --- | --- |
| ticker_symbol | string | obrigatório; 1–64 caracteres | Código publicado; espaços externos são removidos e letras ASCII são normalizadas para maiúsculas. Ticker não é identidade permanente. |
| data_revision | UUID | opcional | Sem cursor, seleciona essa revisão publicada; quando omitido, escolhe a revisão ativa uma vez e a informa em `meta`. Com cursor pode ser omitido; se informado, deve coincidir com a revisão do cursor. |
| cursor | string | opcional; máximo 1.024 caracteres | Valor opaco URL-safe sem padding emitido em `checkpoint`, vinculado a ticker, revisão, versão do schema e chave `(trade_date, id)`. Retoma exclusivamente depois da chave. |
| include_internal | boolean | `false` | `true` inclui `internal.version_id` e `internal.business_hash` dentro de cada cotação. Quando falso, `internal` é omitido. Não amplia autorização. |
| If-None-Match | header | opcional | ETag forte da representação pronta na mesma codificação. Correspondência retorna `304` sem corpo. |
| Accept-Encoding | header | opcional | `gzip` aceita compressão se a representação gzip estiver pronta. A primeira geração pode ser NDJSON sem compressão. |

Não envie `limit`, `offset`, intervalo de datas ou filtros de mercado: a rota é uma exportação integral e rejeita parâmetros desconhecidos com `400`. Não há paginação HTTP de offset nem máximo de itens de lista. Internamente, a geração consulta PostgreSQL em páginas keyset limitadas a 500 fatos.

Exemplo:

```http
GET /api/v1/b3/cotahist/quotes/export?ticker_symbol=PETR4
Authorization: Bearer <read-token>
Accept: application/x-ndjson
Accept-Encoding: gzip
```

`Accept` não altera os filtros; a resposta é `application/x-ndjson; charset=utf-8`. O protocolo tem estes eventos:

| Evento | Campos e semântica |
| --- | --- |
| `meta` | Primeira linha. Informa `schema_version`, ticker normalizado, `data_revision`, `resumed_after` (`null` na primeira chamada) e o aviso nominal obrigatório. |
| `quote` | Um `HistoricalQuote` completo em `data`, com campos COTAHIST, decimais textuais, estado de identidade e proveniência. Consulte [schema COTAHIST](../schemas/cotahist.md). |
| `checkpoint` | Emitido ao final de cada página interna de até 500 fatos. `cursor` retoma exclusivamente após o último `(trade_date,id)`. `records_sent` conta cumulativamente os `quote` desta resposta; em uma retomada começa novamente em zero. |
| `complete` | Marcador final verificável. `record_count` conta os fatos desta resposta; `last_cursor` é o cursor final; `content_sha256` é o SHA-256 dos bytes UTF-8 das linhas anteriores, incluindo o LF de cada uma e excluindo a própria linha `complete`. |
| `error` | Erro seguro se a geração falhar depois do início do corpo; inclui `code`, `message` e `request_id`. Não é seguido por `complete`. |

Exemplo estrutural abreviado; o hash é ilustrativo e não confere com estas linhas:

```ndjson
{"type":"meta","schema_version":1,"ticker_symbol":"EXMP3","data_revision":"00000000-0000-4000-8000-000000000001","resumed_after":null,"disclaimer":"Preços históricos nominais brutos, na moeda e forma de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos."}
{"type":"quote","data":{"id":"00000000-0000-4000-8000-000000000010","trade_date":"2025-01-02","ticker_symbol":"EXMP3","last_price":"30.20","identity":{"instrument_id":null,"status":"unresolved"},"provenance":{"source_file_id":"00000000-0000-4000-8000-000000000011","source_file_type":"COTAHIST_YEARLY","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source_member_id":"00000000-0000-4000-8000-000000000012","source_member_name":"COTAHIST.2025.TXT","record_index":42}}}
{"type":"checkpoint","cursor":"eyJzY2hlbWFfdmVyc2lvbiI6MSwidGlja2VyX3N5bWJvbCI6IkVYTVAzIiwiZGF0YV9yZXZpc2lvbiI6IjAwMDAwMDAwLTAwMDAtNDAwMC04MDAwLTAwMDAwMDAwMDAwMDEiLCJ0cmFkZV9kYXRlIjoiMjAyNS0wMS0wMiIsInF1b3RlX2lkIjoiMDAwMDAwMDAtMDAwMC00MDAwLTgwMDAtMDAwMDAwMDAwMDEwIn0","records_sent":1}
{"type":"complete","record_count":1,"last_cursor":"eyJzY2hlbWFfdmVyc2lvbiI6MSwidGlja2VyX3N5bWJvbCI6IkVYTVAzIiwiZGF0YV9yZXZpc2lvbiI6IjAwMDAwMDAwLTAwMDAtNDAwMC04MDAwLTAwMDAwMDAwMDAwMDEiLCJ0cmFkZV9kYXRlIjoiMjAyNS0wMS0wMiIsInF1b3RlX2lkIjoiMDAwMDAwMDAtMDAwMC00MDAwLTgwMDAtMDAwMDAwMDAwMDEwIn0","content_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
```

### Retomada e integridade

Guarde `data_revision` de `meta`. Após persistir duravelmente todos os `quote` anteriores a um `checkpoint`, grave seu cursor. Se a conexão cair antes do próximo checkpoint, retome com o cursor anterior; a chave exclusiva evita repetir fatos já confirmados. Envie o mesmo ticker. `data_revision` pode ser omitido na retomada, pois a revisão está no cursor; se enviado, precisa coincidir. A revisão ativa atual não substitui a revisão da extração.

Cada resposta retomada tem novo `meta`, `record_count` próprio e checksum próprio. Some `record_count` apenas de segmentos concluídos para obter o total. Calcule SHA-256 incrementalmente sobre os bytes exatos das linhas anteriores ao evento `complete`, preservando LF; não reserialize objetos JSON para calcular o hash. Só marque a extração como concluída se `complete` estiver presente, nenhum `error` tiver ocorrido e `content_sha256` conferir. Sem `complete`, uma falha de rede, JSON inválido ou checksum divergente significa que o segmento não está confirmado.

Para revisão, ticker, `include_internal` e cursor fixos, repetir a chamada retorna o mesmo segmento NDJSON; não há `Idempotency-Key` para essa leitura. A autorização e a elegibilidade REST/`third_party_delivery` são revalidadas em toda chamada, inclusive em cache hit.

### Cabeçalhos, cache e respostas HTTP

| Campo | Comportamento |
| --- | --- |
| `Cache-Control` | `private, max-age=0, must-revalidate`; não encaminhe para CDN/cache público compartilhado. |
| `Vary` | `Authorization, Accept-Encoding`. |
| `ETag` | Hash forte da representação pronta selecionada (NDJSON ou gzip); não é enviado enquanto a geração está em curso. |
| `Content-Length` | Presente somente para artefato pronto; fluxo em geração não declara tamanho final. |
| `Content-Encoding` | `gzip` quando a cópia comprimida pronta for selecionada. Primeira geração pode ser sem compressão. |
| `If-None-Match` | `304` sem corpo somente quando existe representação pronta idêntica. No cache miss, a resposta segue como `200` em fluxo. |

Antes do início do corpo, erros usam o envelope da API: `400` (parâmetro desconhecido, ticker/cursor/revisão inválida), `401` (Bearer ausente ou inválido), `403` (elegibilidade REST/`third_party_delivery`), `404` (ticker sem fatos na revisão), `500` (falha interna) e `503` (credenciais, revisão, PostgreSQL ou storage indisponível). Depois dos headers, HTTP não troca o status: leia evento `error` e trate corpo truncado como incompleto. Um `200` sem `complete` não representa sucesso.

O artefato interno é derivado, reconstruível e dura até 30 dias; PostgreSQL continua como fonte de verdade. O serviço gera páginas keyset de 500 fatos e transmite enquanto o artefato é escrito. A implementação usa `B3_PROCESSED_DATA_DIR/quote-exports`; instâncias que compartilham cache precisam compartilhar o filesystem persistente. A meta de primeira cotação abaixo de um segundo para 50 consumidores requer benchmark representativo e não é SLA publicado.

## GET /api/v1/b3/cotahist/series

Usa os mesmos filtros e envelope da consulta de fatos, mas exige uma seleção de série sem múltiplas linhas incompatíveis por pregão. Se mercado, distribuição ou prazo não desambiguarem a série, retorna 409 AMBIGUOUS_SERIES. A rota não faz forward-fill e não converte PREULT em preço genérico de fechamento.

## Interpretação obrigatória

A API publica valores nominais brutos na moeda e base de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos. A resposta conserva o aviso, FATCOT, unidade, proveniência e a situação de identidade. Consulte [schema COTAHIST](../schemas/cotahist.md) antes de construir séries financeiras.
