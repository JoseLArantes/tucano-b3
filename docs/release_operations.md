# Operação do primeiro lançamento

Status: plataforma 0.0.1 qualificada com PostgreSQL 16 e fixture sintética; canais externos continuam desabilitados até a evidência de elegibilidade por finalidade. Consultar o [plano 0.0.1](0.0.1-plan.md) e a [elegibilidade de fontes](release_source_eligibility.md).

## Inicialização e papéis

Aplicar a cadeia de migrations com `tucano-b3 migrate`, usando `B3_MIGRATION_DATABASE_URL` e a credencial de migração. API, worker e MCP verificam versão/checksums do schema e não executam DDL em seus próprios processos. No Compose local, `b3_migrate` é um serviço de tarefa única: `docker compose up -d` aguarda sua conclusão bem-sucedida antes de iniciar API e worker. Para executar a migration separadamente, rode `docker compose run --rm b3_migrate`. Em produção, preserve a mesma ordem explícita no pipeline de release; não conceda DDL à credencial de runtime.

Usar credenciais separadas para migração, escrita administrativa/ingestão e leitura analítica. O papel do MCP precisa ler os metadados SQLx de schema e as relações usadas pelos query services, sem INSERT/UPDATE/DELETE/DDL. Credenciais Bearer de leitura e administração devem ser distintas; exceções públicas não incluem operações de ingestão.

Não há fontes habilitadas por seed. Registrar canal, formato, período, publicação, evidência por finalidade e calendário aplicável antes de habilitar o fluxo. Uma configuração local não constitui autorização da B3.

## Aquisição e processamento

O diretório configurado por `B3_RAW_DATA_DIR` contém:

- `imports/`: entradas locais permitidas, identificadas por caminho relativo confinado.
- `tmp/`: transferências em construção; arquivos parciais nunca são publicados como raw íntegro.
- `raw/`: conteúdo endereçado por SHA-256, incluindo members extraídos e containers aninhados.

O transporte HTTPS valida host exato, porta, redirects e endereços públicos resolvidos, fixando os endereços na conexão. Tokens temporários não entram nos erros públicos. Importação local não permite navegar fora de `imports/` por caminho ou symlink.

O inventário conserva caminho de aninhamento, ordem da entrada, pai, hash e tamanho; diretórios/containers/entradas não selecionadas recebem motivo. Parsing recebe arquivos em disco e callbacks de registros. A descoberta usa HEAD; não recorre a GET pesado quando HEAD não é suportado.

O fluxo de duas etapas com token não é anunciado como homologado: exige canal/schema demonstrados. Apesar de existirem leitores e fixtures exploratórios para dados de registro, BVBG.028 e Tradable Security List estão `deferred` no catálogo 0.0.1: seus contratos não oferecem uma data interna por registro para aplicar os cortes do primeiro release. O serviço rejeita a criação de jobs de ingestão para essas famílias; não as habilite por configuração. Não habilitar automaticamente um adaptador desconhecido para obter uma resposta vazia.

## Backup consistente de SQL e raw

O cache de exportações em `B3_PROCESSED_DATA_DIR/quote-exports` é derivado e reconstruível; não é incluído no backup consistente de SQL+raw nem necessário para restaurar o histórico. As entradas expiram após 30 dias. Após perda do volume, manifests sem arquivo são marcados como falha e a próxima chamada reconstrói o artefato. Em várias instâncias da API, o caminho precisa ser um filesystem persistente compartilhado. O Compose de desenvolvimento usa um volume local e não fornece armazenamento multi-host.

`scripts/release_snapshot.py` usa apenas Python padrão como ferramenta operacional; a aplicação continua em Rust. PostgreSQL é acessado pelos clientes do container Compose, sem imprimir senhas ou aceitar credenciais em URL na linha de comando.

O backup exporta um snapshot PostgreSQL de leitura e mantém a transação viva durante a captura. O dump, os localizadores raw e a consulta de referência usam esse mesmo snapshot. Como os bytes raw são imutáveis, a cópia pode acontecer sem bloquear ingestões; limpeza não deve remover arquivos referenciados enquanto um backup está em andamento.

Exemplo a partir da raiz do repositório, substituindo o banco e os diretórios pelo ambiente autorizado:

```bash
python3 scripts/release_snapshot.py backup \
  --database release_fixture \
  --raw-root /caminho/isolado/raw-root \
  --destination /caminho/isolado/backups/referencia-01
```

O destino deve ser novo e ficar fora do raw root. O manifesto contém checksum do dump, hashes/tamanhos de cada conteúdo referenciado, revisão ativa e digest ordenado dos fatos/versões/vínculos selecionados. `INCOMPLETE` identifica uma captura interrompida e impede restauração. Um dump sem bytes recuperáveis não comprova replay.

Backups contêm evidência e dados operacionais; manter controle de acesso e retenção compatíveis com a fonte. O programa não apaga raw, dados de domínio ou backups antigos.

## Restore em destino novo

```bash
python3 scripts/release_snapshot.py restore \
  --database restore_fixture \
  --backup /caminho/isolado/backups/referencia-01 \
  --raw-root /caminho/isolado/restore-raw
```

O comando recusa qualquer nome de banco já existente e raw root não vazio. Verifica dump e bytes antes de criar o banco, restaura sem ownership/ACL antigos, remapeia apenas localizadores físicos e compara a revisão/digest de referência. Não inicializa API/worker nem publica uma revisão nova. Falha conserva o destino novo para diagnóstico; não remove nem sobrescreve um banco existente.

Depois do restore, conferir papéis de leitura/escrita e evidências de fonte antes de iniciar consumidores. Retomada de jobs é administrativa: lease antiga não concede direito de confirmação a um processo substituído.

## Limites e medição

Os limites de transferência, bytes por member, expansão total, razão de expansão, quantidade de entradas, profundidade e timeout são controles de segurança, não resultados de desempenho. O runtime deve reservar manutenção e limitar trabalho pesado/queries. A adequação dos valores depende do corpus e hardware medidos.

Toda medição registra commit, hardware, PostgreSQL/configuração, corpus/hashes, fases incluídas, quantidade de registros, limites e concorrência. Separar download, inventário, parser, staging/persistência, publicação e consulta. Um benchmark de parser não representa throughput completo.

Relatórios sintéticos devem permanecer identificados como sintéticos. Medir RSS, duração, conexões e WAL do processo/instância corretos; consumo do cliente Docker não é memória do worker. WAL compartilhado pode incluir atividade de outros processos e precisa ser qualificado. Metas do PRD não se tornam resultados por estarem configuradas.

## Evidências desta implementação

A prova SQL/raw foi executada em 06/11/2026, PostgreSQL 16 em bancos exclusivos gerados para o ensaio. O fixture SQL sintético continha uma revisão publicada, um fato nominal e um conteúdo raw. Backup e restore reproduziram a revisão `b578b66f-2921-4990-a858-65f7d7c7d505`, um registro selecionado e o digest `deb66a73968749c615af6c09c4a3bcfd6e7113c7ddcae56b0cd6784b8d0e0baa`.

Foram comprovados remapeamento de localizadores, recusa de destino existente e recusa de raw corrompido antes de criar o banco. Os três bancos temporários desse ensaio foram removidos; nenhum banco operacional foi alterado. Artefatos temporários ficaram fora do repositório, em diretório `b3_snapshot_proof_4c98bx42` do sistema.

No ensaio repetido, o backup levou 0,918 s e o restore 1,677 s, medidos por relógio monotônico do cliente incluindo chamadas Docker/psql. **Esses números descrevem somente um fixture de um registro**: não medem ingestão, memória do worker, WAL atribuível ao pipeline, latência REST ou throughput oficial. O ensaio não inicializou API/MCP sobre o restore.

## Benchmark sintético integrado 0.0.1

Data: **06/11/2026**. Esta execução valida o fluxo da plataforma com dados sintéticos, não com publicações B3. A fixture imita linhas de largura fixa de COTAHIST para exercitar parser, persistência, revisão e consulta; não comprova schema oficial, conteúdo de pregão real, completude ou autorização de uso. O resultado demonstra os recursos e comportamento de importação, não disponibilidade de dados externos.

### Ambiente e fixture

- API, worker e migrate construídos a partir da árvore de código qualificada nesta entrega.
- Docker Engine 29.8.2, arquitetura `aarch64`, 2 CPUs e cerca de 8 GiB de RAM reportados pelo runtime.
- PostgreSQL **16.15** em projeto Compose isolado e descartável via [`docker-compose.benchmark.yml`](../docker-compose.benchmark.yml); sem acesso ao banco operacional.
- Worker em lote de 500 registros / 4 MiB; pool de leitura da API limitado a 8 conexões; lease de worker de 120 s.
- Uma fixture COTAHIST-shaped: 20.000 detalhes para `2025-01-02`, um member, 4.920.492 bytes, SHA-256 `a001f2f338f8393aeb3b9eeb7620e3130073ecc692a5b4313b9740e192935c67`. O canal e a elegibilidade sintéticos foram criados apenas no banco temporário com a referência `SYNTHETIC BENCHMARK ONLY`.
- Import limpa publicou automaticamente a revisão `8352b297-cda8-41c0-a23a-3ba229ac4c1b`: 20.000 fatos, zero rejeições e zero conflitos. Todos ficaram `unresolved`, demonstrando que incerteza cadastral não descarta o preço.

### Resultados observados

| Fase/medida | Resultado | Como interpretar |
| --- | ---: | --- |
| Aquisição local e SHA-256 | 71 ms | Não mede download HTTPS nem rede B3 |
| Inventário de um member simples | 38 ms | Sem ZIP ou expansão |
| Parsing + staging | 22,422 s | 20.000 registros; maior parte do processamento |
| Worker até sucesso e publicação automática | ~22,67 s | Aproximadamente 882 registros/s; sem espera em fila |
| Pico RSS do worker, amostrado | 6,852 MiB | Amostras de `docker stats`, não profiling de alocação |
| Pico CPU do worker, amostrado | 2,18% | Maior amostra observada, não média de CPU |
| Aumento de WAL PostgreSQL | 45.771.358 bytes | `pg_stat_wal` antes/depois na instância dedicada durante o worker |
| Banco após ingestão | ~36 MB | Tamanho total do database, incluindo schema e índices |
| REST concorrente | 160 requests, 8 clientes | Cada resposta limitada a 100; total de 16.000 linhas |
| Latência REST | p50 605,27 ms; p95 814,15 ms | Uma rota e consulta; sem cache externo |
| Throughput REST | 13,60 requests/s | Carga executada após ingestão |
| Backup SQL+raw | 2,12 s | Um raw e uma revisão ativa; diretório montado pelo mesmo caminho absoluto no host e containers |
| Restore | ~3,0 s | Banco novo; revisão e 20.000 linhas de referência verificadas |

O backup preservou a revisão `8352b297-cda8-41c0-a23a-3ba229ac4c1b` e o digest `4883c1bd92db0bb4909efc5f0ac218469b329eeb1c69709d3c2335d373bfb66b`. Depois do restore, a API iniciou contra o banco restaurado, health retornou HTTP 200 e a consulta retornou 20.000 fatos com proveniência e o mesmo identificador de revisão.

### Reprodução

Use sempre um projeto Compose exclusivo. O arquivo de benchmark não publica a porta PostgreSQL no host; o volume é local e descartável.

```bash
COMPOSE_PROJECT_NAME=tucano-b3-benchmark docker compose -f docker-compose.benchmark.yml up -d b3_postgres
docker compose -f docker-compose.benchmark.yml -p tucano-b3-benchmark ps
```

Aplique as migrations na base isolada. Gere a fixture e confira o checksum:

```bash
mkdir -p /tmp/tucano-b3-benchmark/imports/bench
python3 scripts/generate_synthetic_cotahist.py \
  --records 20000 \
  --trade-date 2025-01-02 \
  --output /tmp/tucano-b3-benchmark/imports/bench/cotahist.txt \
  --command-file /tmp/tucano-b3-benchmark/ingest.json
shasum -a 256 /tmp/tucano-b3-benchmark/imports/bench/cotahist.txt
```

Cadastre somente no banco descartável um canal local `synthetic-benchmark-2025`, tipo `COTAHIST_YEARLY`, perfil `cotahist_latin1`, período 2025 e caminho `bench/cotahist.txt`. Registre elegibilidade sintética para os propósitos requeridos com a referência literal `SYNTHETIC BENCHMARK ONLY`; essa evidência nunca deve ser copiada a uma fonte real. Monte `/tmp/tucano-b3-benchmark` no API/worker pelo mesmo caminho absoluto e configure `B3_RAW_DATA_DIR` para esse caminho, permitindo que o backup localize os arquivos raw.

Submeta `ingest.json` pelo caminho administrativo, execute `worker --once --lane maintenance` e aguarde o job. Um import limpo publica automaticamente ao concluir os gates; não envie um job de publicação separado. Inicie a API contra o mesmo banco e execute a carga versionada:

```bash
python3 scripts/benchmark_api_reads.py \
  --base-url http://127.0.0.1:18124 \
  --token <read-token-temporario> \
  --requests 160 --concurrency 8 --limit 100
```

Para repetir backup/restore, defina `COMPOSE_FILE=docker-compose.benchmark.yml` e `COMPOSE_PROJECT_NAME=tucano-b3-benchmark` ao executar `scripts/release_snapshot.py backup/restore`, de modo que o cliente Docker use o PostgreSQL isolado. Encerre somente esse projeto e seu volume descartável ao terminar.

### Limites dos resultados

Esta medição cobre um arquivo sintético de 4,9 MB, uma família, um member e uma data, sem identidade cadastrada. Não representa aquisição de publicação B3, arquivos com sobreposição, volume anual, longa retenção, concorrência entre ingestão e API ou escala de produção. RSS/CPU são amostras de containers, e WAL vale apenas para a instância dedicada. **Estes resultados não fixam budgets de produção**; ajuste capacidade com corpora autorizados e hardware de implantação antes de ativar canais reais.
