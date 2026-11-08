---
status: accepted
date: 2026-11-08
---

# Exportação COTAHIST longa por NDJSON fixado em revisão

Para entregar o histórico integral de um ticker sem materializar uma resposta JSON gigante nem alterar o contrato paginado de `/cotahist/quotes`, adicionamos `/cotahist/quotes/export` como fluxo HTTP finito `application/x-ndjson`. Ele emite `meta`, linhas tipadas `quote`, checkpoints retomáveis e `complete`; a revisão, o ticker, a versão do schema, `include_internal` e o cursor determinam a identidade do artefato. A chave `(trade_date, id)` ordena os fatos e permite retomada exclusiva sem repetir dados.

PostgreSQL continua autoritativo. O miss usa keyset pagination em lotes limitados e transmite durante a geração; a tabela `b3_quote_export_artifacts` coordena builders por lease e guarda estado, checksum, tamanho e expiração. Artefatos completos ficam no filesystem de `B3_PROCESSED_DATA_DIR/quote-exports`, são comprimidos em gzip para cache hits e expiram após 30 dias. Cada request verifica Bearer e elegibilidade REST/`third_party_delivery` antes de ler o manifesto ou arquivo. Respostas HTTP são privadas, variam por `Authorization`/`Accept-Encoding` e só oferecem ETag quando a representação está completa.

Essa é uma exceção deliberada e estreita à regra de medir antes de adicionar cache persistente: evita repetir a exportação longa para chamadas idênticas e atende a necessidade de um endpoint de transferência integral. Não introduz Redis, Kafka, fila de exportação, read replica ou outro datastore. O filesystem atual é compartilhável entre instâncias somente quando o deployment fornece um volume comum; a configuração Compose é local e não oferece storage multi-host. Antes de afirmar a meta de p95 < 1 s para primeira cotação sob 50 consumidores, medir cold/warm cache, 50 mesmos tickers e 50 tickers distintos, ingestão concorrente, recursos do PostgreSQL e disco. Adotar storage de objetos, replica de leitura ou limites de concorrência adicionais exige evidência desse benchmark.

O endpoint é um snapshot histórico finito, não um feed live. COTAHIST permanece nominal, sem ajustes retroativos, e cada cotação conserva identidade e proveniência. Cliente só considera o fluxo íntegro após validar `complete` e `content_sha256`; HTTP cache compartilhado é proibido para essas respostas autenticadas.
