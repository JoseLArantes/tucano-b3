---
title: Operação da ingestão
---

# Operação do serviço

A operação separa descoberta de publicações, aquisição e processamento pesado, revisão e publicação. A API aceita comandos administrativos duráveis; o worker realiza o trabalho fora da requisição HTTP e grava contadores e estados confirmados no PostgreSQL.

## Princípios

- A descoberta leve registra publicações potenciais sem baixar datasets.
- Jobs são duráveis e compartilhados por API, CLI e worker.
- Prioridade de manutenção mantém capacidade reservada para atualizações regulares.
- Cancelamento é registrado como pedido durável e aplicado nos limites seguros do executor.
- Arquivos brutos e inventário de members são preservados para inspeção e reprocessamento.
- Uma revisão publicada não é trocada por candidato incompleto.
- Erro de infraestrutura ou de arquivo permanece visível; não vira uma resposta vazia bem-sucedida.

## Fluxo geral

    descoberta → pendência → comando administrativo → aquisição/inventário → parsing em lotes → gates de qualidade → publicação automática da revisão limpa

Uma revisão com rejeições ou conflitos não é publicada automaticamente e termina com issues para decisão administrativa. O fluxo também pode terminar com falha ou cancelamento. A consulta de dados não inicia esse fluxo.

## Cache de exportação COTAHIST

`GET /api/v1/b3/cotahist/quotes/export` usa PostgreSQL como fonte de verdade e mantém uma cópia derivada em `B3_PROCESSED_DATA_DIR/quote-exports`. A chave de cache é derivada de revisão, ticker, versão do schema, projeção `include_internal` e cursor. Assim, um segmento retomado não colide com o arquivo integral nem com outra projeção.

### Caminho de uma chamada

1. A API autentica a credencial Bearer e resolve uma revisão publicada. Mesmo em cache hit, revalida elegibilidade REST e `third_party_delivery` e faz uma consulta indexada de existência do ticker na revisão. Isso não lê e serializa o histórico inteiro.
2. Para um artefato pronto, consulta o manifesto PostgreSQL e transmite o arquivo completo (gzip se essa representação estiver pronta e for aceita). A autorização e elegibilidade são novamente aplicadas em cada requisição.
3. Em cache miss, uma linha de manifesto e lease no PostgreSQL elege um builder por chave. Consumidores simultâneos da mesma chave acompanham o arquivo parcial que o builder grava; uma instância que perdeu o lease não pode sobrescrever o arquivo temporário de outra geração porque cada geração usa caminho associado ao seu token de lease.
4. O builder consulta a revisão em páginas keyset de até 500 cotações, em ordem `(trade_date,id)`. Cada página é serializada como eventos NDJSON, gravada e liberada para leitura enquanto a consulta continua. Checkpoints permitem ao consumidor retomar depois da última página confirmada.
5. Ao terminar, o processo grava `complete`, calcula e confirma os digests, sincroniza o arquivo, renomeia o temporário para o artefato final e atualiza o manifesto. A representação gzip é derivada depois; enquanto ela não está pronta, o consumidor recebe NDJSON sem compressão.

O manifesto mantém status, lease, checksum, tamanho, contagem e expiração de 30 dias. Uma falha ou ausência do arquivo invalida a entrada pronta e permite reconstruí-la. A expiração remove artefatos derivados; não remova tabelas, fatos ou revisões PostgreSQL para limpar cache. O volume de exportações não contém fonte oficial e não é necessário para restaurar o histórico.

### Topologia e limites de escala

Todas as instâncias que compartilham o manifesto também precisam ler e gravar o mesmo filesystem persistente montado em `B3_PROCESSED_DATA_DIR`. O Compose de desenvolvimento usa um volume local e não fornece storage multi-host nem integração com object storage. Portanto, uma implantação horizontal em vários hosts precisa fornecer filesystem compartilhado compatível; um serviço de objeto ainda não está implementado. PostgreSQL mantém a coordenação durável por lease; Redis ou uma fila em memória não participam do protocolo.

Cache hits não eliminam todas as consultas ao banco: revisão, elegibilidade, existência indexada e estado do manifesto continuam validados. O caminho de cache evita reler e reconstruir todas as cotações. Misses consultam o banco em páginas limitadas e compartilham a mesma geração entre consumidores da mesma chave. Não há SLA de latência publicado. A meta de primeira cotação inferior a um segundo para 50 consumidores só pode ser anunciada após benchmark reproduzível em corpus, hardware, topologia e padrões de cache representativos.

Como o endpoint é finito, consumidores só confirmam uma extração quando recebem `complete` e validam `content_sha256`. Evento `error`, falha de transporte ou término sem `complete` significa corpo parcial; use o último `checkpoint` persistido para retomar.

## Segurança e autorização

POST de comando, cancelamento, retry, scan e reconhecimento de pendência exige Bearer administrativo. GET de jobs, sync-runs, arquivos, members, pendências e revisões também exige administração. Não publique credenciais administrativas a consumidores da API.

## Antes de habilitar fonte

O endpoint de capacidades informa famílias implementadas, adiadas e elegibilidade para REST e entrega a terceiros. Um canal configurado não basta: a finalidade precisa ter evidência válida de elegibilidade. Nenhuma fonte é habilitada por seed. Para procedimentos de runtime e retenção, consulte também [release_operations.md](https://github.com/JoseLArantes/b3-tucano/blob/main/docs/release_operations.md).
