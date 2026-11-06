---
title: Schemas de cadastro
---

# Schemas de evidência cadastral

InstrumentPage contém items, total, limit, offset e snapshots. Cada InstrumentEvidence documenta uma observação em uma publicação e pode conter:

| Campo | Significado |
| --- | --- |
| instrument_id | UUID canônico, nulo sem evidência forte admissível |
| snapshot_id | Fotografia que sustenta os atributos |
| reference_date | Referência da fotografia; não presume intervalo de validade |
| security_id, security_source | Código publicado e seu namespace/qualificador |
| market_identifier_code, market_code | Qualificadores de mercado publicados |
| ticker_symbol, isin | Identificadores observados na fonte |
| asset_description | Descrição oficial, sem enriquecimento CVM |
| instrument_type, security_category, asset | Classificação e códigos da fonte, sem heurística de emissor |
| segment_code, cfi_code | Segmento e classificação CFI quando publicados |
| instrument_status | Situação do instrumento na fotografia |
| maturity_date | Data de vencimento sem valor sentinela |
| tradable_indicator | Indicador TSL, quando disponível |
| trading_start_date, trading_end_date | Datas publicadas, sem inferir a partir de ausência |
| provenance | Referência à origem do registro |

## Evidência de identificadores

InstrumentIdentifierEvidence inclui instrument_id, identifier_type, identifier_value, market_code, valid_from e valid_to, snapshot_id, reference_date e provenance. Janela nula significa que a fonte não sustentou o limite; o consumidor não deve preencher por suposição.

Famílias não são mescladas silenciosamente. Para uma fotografia específica, fixe snapshot_id. Quando várias fontes trazem valores distintos, conserve as observações lado a lado e investigue os qualificadores e a fonte.
