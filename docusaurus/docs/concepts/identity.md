---
title: Identidade temporal
---

# Identidade temporal de instrumentos

A identidade de instrumento deriva do registro B3 e das evidências cadastrais qualificadas. A consulta preserva os atributos publicados em cada fotografia e não presume que um valor atual de ticker ou cadastro tenha sido válido em todo o passado.

## Resolução

A precedência definida pelo produto começa por security_id. ISIN, ticker, mercado e data fornecem evidência adicional conforme a família de origem e o período. A ligação usa qualificadores e janelas de validade sustentadas pelos dados disponíveis. Um ticker isolado pode ser reutilizado ou existir em mercados distintos.

O contrato representa o estado de identidade como:

| Estado | Interpretação |
| --- | --- |
| resolved | Evidência temporal sustentou uma associação inequívoca |
| unresolved | Não há evidência suficiente para associar o fato |
| ambiguous | Mais de uma associação permanece possível |

Preço sem associação continua válido como fato publicado, com instrument_id nulo e estado de identidade explícito. Não escolha arbitrariamente um dos candidatos.

## Fotografia e validade

reference_date é a data associada à fotografia. Ela não prova início ou fim de validade contínua do ticker. Os endpoints cadastrais expõem snapshot_id, fonte, qualificadores e proveniência para que o consumidor possa avaliar a evidência sem confundir publicação com janela temporal.

## Consultas

- Use ticker_symbol para buscar ocorrências com aquele código publicado.
- Use instrument_id para consultar fatos que já têm uma associação temporal sustentada.
- Use ISIN e qualificadores de mercado quando forem conhecidos.
- Use /api/v1/b3/instruments/&#123;id&#125;/identifiers para examinar valores e janelas explicitamente registradas.
- Quando a série continuar ambígua, refine filtros de mercado, distribuição e prazo. A série COTAHIST retorna conflito em vez de inventar uma seleção.

Rebuilds são operações administrativas com acompanhamento; não são executados implicitamente durante consultas.
