---
title: Revisões e consistência
---

# Revisões e consistência de consultas

Uma revisão identifica o conjunto controlado selecionado para consulta. Ela separa fatos já disponíveis do candidato em preparação e evita que duas páginas de uma mesma leitura mudem de conjunto durante a paginação.

## Revisão em consultas

O endpoint de cotações retorna data_revision. Envie esse UUID nas páginas seguintes para repetir o mesmo conjunto. Quando omitido, o serviço seleciona a revisão ativa em cada requisição. Use sempre o valor retornado para leituras paginadas consistentes.

    /api/v1/b3/cotahist/quotes?ticker_symbol=PETR4&limit=100&offset=0
    /api/v1/b3/cotahist/quotes?ticker_symbol=PETR4&data_revision=UUID_DA_RESPOSTA&limit=100&offset=100

## Estados de revisão

Revisões e snapshots podem aparecer como draft, ready, published ou invalid:

- **draft**: construção ainda não selada;
- **ready**: validações de preparação permitem uma promoção administrativa;
- **published**: revisão explicitamente publicada e consultável quando elegível;
- **invalid**: candidata não pode ser publicada no estado atual.

Uma fotografia cadastral também tem revisão/identificador próprio; seu período é a data de referência da fonte, não uma janela de validade presumida.

## Cobertura não é continuidade

first_observed e last_observed descrevem registros selecionados. Esses limites não provam que todos os pregões entre eles estejam cobertos. O contrato também informa rejeições, identidade pendente e estado do calendário conhecido; não infere percentual de completude sem calendário demonstrado.

## Publicação

A promoção é uma ação administrativa observável e executada por job. A revisão publicada anteriormente continua disponível até que a candidata passe pelos gates e seja promovida. A inspeção de uma revisão não inicia ingestão e não publica a candidata.
