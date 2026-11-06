---
title: Cadastro de instrumentos
---

# Cadastro B3 e evidências de instrumento

O cadastro é uma fonte de identidade, não uma substituição das cotações. Cada observação identifica sua fotografia, família, data de referência, qualificadores e proveniência.

## Famílias

- **BVBG.028.02 / InstrumentsConsolidated**: publicação cadastral estruturada cuja observação deve ser interpretada conforme sua categoria e qualificador.
- **Tradable Security List (TSL)**: lista de instrumentos negociáveis, mantida como família distinta.
- **InstrumentsConsolidatedFile**: arquivo CSV cadastral complementar definido no escopo e no material de referência do projeto; sua existência no escopo não implica exposição em todos os ambientes.

Um endpoint que consulta uma família não deve preencher campos ausentes com informação de outra família. Em particular, GET /api/v1/b3/tradable-securities filtra para TSL e não cai para BVBG.028.

## Fotografias

GET /api/v1/b3/instruments retorna evidências cadastrais publicadas. Cada item pode conter instrument_id, snapshot_id, reference_date, security_id e namespace, mercado, ticker, ISIN, descrição, categoria, status, datas de negociação e provenance.

reference_date identifica a fotografia. Ela não prova quando um identificador começou ou deixou de ser válido. Para identificar janelas explicitamente registradas, use /api/v1/b3/instruments/&#123;id&#125;/identifiers.

## Correspondência e limites

Um código de security_id é qualificado pelo namespace da fonte. ISIN e ticker ajudam conforme a evidência temporal, mas ticker isolado não é identidade permanente. O sistema preserva ausência de vínculo e colisões sem escolher um registro arbitrariamente.

Não são usados dados CVM, feeds de corretoras, enriquecimento externo, nem identificadores de emissor como âncora da relação.
