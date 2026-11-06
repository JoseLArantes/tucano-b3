---
title: Primeira consulta
---

# Primeira consulta COTAHIST

Este exemplo consulta fatos históricos. Substitua o ticker e as datas pelo recorte desejado e use um token de leitura configurado no serviço.

    curl --get 'http://localhost:8010/api/v1/b3/cotahist/quotes' \
      --header 'Authorization: Bearer SEU_TOKEN_DE_LEITURA' \
      --data-urlencode 'ticker_symbol=PETR4' \
      --data-urlencode 'trade_date_from=2025-01-01' \
      --data-urlencode 'trade_date_to=2025-12-31' \
      --data-urlencode 'market_type=010' \
      --data-urlencode 'limit=100' \
      --data-urlencode 'offset=0'

Datas são inclusivas. market_type preserva os zeros à esquerda. O ticker deve ser interpretado junto a mercado, data e demais discriminadores; sozinho, ele não é uma chave de instrumento permanente.

## O que conferir na resposta

- items contém os fatos e os campos de negócio de cada registro;
- total, limit e offset descrevem a página;
- data_revision identifica a revisão consultada e pode ser enviada em páginas seguintes;
- disclaimer mantém o aviso de preços nominais sem ajuste;
- coverage comunica o recorte observado, rejeições, identidade incerta e estado do calendário;
- identity informa se a ligação cadastral foi resolvida, permaneceu sem resolução ou foi ambígua;
- provenance identifica arquivo, member e índice original do registro.

Valores financeiros e quantidades são serializados como strings decimais. Preserve esse formato ao armazenar ou calcular para não perder escala ou precisão.

Para uma série que deva representar uma única linha por dia, consulte /api/v1/b3/cotahist/series. Se mercado, distribuição ou prazo deixarem mais de uma ocorrência por pregão, a rota responde 409 AMBIGUOUS_SERIES. Acrescente filtros para tornar a seleção inequívoca.

    curl --get 'http://localhost:8010/api/v1/b3/cotahist/quotes' \
      --header 'Authorization: Bearer SEU_TOKEN_DE_LEITURA' \
      --data-urlencode 'ticker_symbol=PETR4' \
      --data-urlencode 'trade_date_from=2025-01-01' \
      --data-urlencode 'limit=10'

Veja [COTAHIST](../api-endpoints/cotahist.md) para todos os filtros, erros e formatos, ou consulte o contrato gerado pelo serviço em /openapi.json.
