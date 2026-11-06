---
title: Endpoints de cadastro
---

# Endpoints de cadastro

As rotas cadastrais estão documentadas, mas a capacidade de ingestão BVBG.028/Tradable Security List está `deferred` no `0.0.1`. Sem fotografias publicadas, a consulta informa indisponibilidade; elas não devem ser usadas como prova de identidade histórica. Cotações válidas continuam consultáveis com identidade `unresolved` quando necessário.

## Filtros compartilhados

GET /api/v1/b3/instruments e GET /api/v1/b3/instruments/&#123;id&#125; aceitam:

| Parâmetro | Significado |
| --- | --- |
| instrument_id | UUID canônico, se resolvido |
| query | Busca textual por ticker, ISIN, código ou descrição |
| ticker_symbol | Correspondência exata ao ticker publicado |
| security_id | Código B3 junto à família/namespace correspondente |
| isin | ISIN da evidência |
| snapshot_id | Fixa uma fotografia para paginação |
| source_file_type | Restringe a família; não aplica fallback |
| limit | 100 por padrão, de 1 a 1000 |
| offset | 0 por padrão, não negativo |

### GET /api/v1/b3/instruments

Retorna uma página de evidências de cadastro com total, limit, offset e snapshots considerados. Itens são ordenados deterministicamente por fotografia, member e linha. Famílias e snapshots diferentes podem trazer observações distintas para o mesmo instrumento. Fixe snapshot_id ao paginar uma fotografia.

### GET /api/v1/b3/instruments/&#123;id&#125;

Retorna todas as observações cadastradas atualmente publicadas para o UUID canônico. O filtro instrument_id, se enviado, deve corresponder ao UUID da rota; caso contrário, o pedido é inválido. Não há seleção arbitrária de uma fotografia.

### GET /api/v1/b3/instruments/&#123;id&#125;/identifiers

Lista identificadores registrados para o UUID com tipo, valor, mercado, valid_from, valid_to, snapshot_id, reference_date e proveniência localizada. Paginação padrão 100/0, máximo 1000. A fotografia não é convertida em intervalo de validade presumido.

### GET /api/v1/b3/tradable-securities

Retorna a família Tradable Security List usando os filtros de cadastro. A consulta não usa BVBG.028 como fallback e não fabrica valores para campos ausentes na projeção. Se família ou snapshot não estiverem elegíveis/publicados, a resposta indica a condição em erro.

## Respostas e interpretação

Falha de filtro/paginação retorna 400; Bearer ausente/inválido, 401; fonte sem elegibilidade, 403; ausência de evidência, 404; conflito de seleção, 409; falha de dependência, 500; nenhuma fotografia publicada, 503 quando aplicável.

Os atributos são os publicados na fonte. asset ou security_id não são automaticamente identificadores de emissor; reference_date não inicia uma validade; null não deve ser preenchido com outra família sem regra de correspondência explícita.
