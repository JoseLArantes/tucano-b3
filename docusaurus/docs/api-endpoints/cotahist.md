---
title: Endpoints COTAHIST
---

# Endpoints de cotações COTAHIST

Ambos os endpoints usam Bearer de leitura, não iniciam aquisição e consultam uma revisão publicada elegível.

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

## GET /api/v1/b3/cotahist/series

Usa os mesmos filtros e envelope da consulta de fatos, mas exige uma seleção de série sem múltiplas linhas incompatíveis por pregão. Se mercado, distribuição ou prazo não desambiguarem a série, retorna 409 AMBIGUOUS_SERIES. A rota não faz forward-fill e não converte PREULT em preço genérico de fechamento.

## Interpretação obrigatória

A API publica valores nominais brutos na moeda e base de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos. A resposta conserva o aviso, FATCOT, unidade, proveniência e a situação de identidade. Consulte [schema COTAHIST](../schemas/cotahist.md) antes de construir séries financeiras.
