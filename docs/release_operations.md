# Operação do primeiro lançamento

Status: qualificação em andamento. Este guia não libera publicação externa; consultar o [plano 0.0.1](0.0.1-plan.md) e a [elegibilidade de fontes](release_source_eligibility.md).

## Inicialização e papéis

Aplicar a cadeia de migrations explicitamente com `tucano-b3 migrate`, usando a credencial de migração. API, worker e MCP verificam versão/checksums do schema e não executam DDL na inicialização. O serviço Compose `b3_migrate`, no profile `tools`, usa a imagem da aplicação para essa operação.

Usar credenciais separadas para migração, escrita administrativa/ingestão e leitura analítica. O papel do MCP precisa ler os metadados SQLx de schema e as relações usadas pelos query services, sem INSERT/UPDATE/DELETE/DDL. Credenciais Bearer de leitura e administração devem ser distintas; exceções públicas não incluem operações de ingestão.

Não há fontes habilitadas por seed. Registrar canal, formato, período, publicação, evidência por finalidade e calendário aplicável antes de habilitar o fluxo. Uma configuração local não constitui autorização da B3.

## Aquisição e processamento

O diretório configurado por `B3_RAW_DATA_DIR` contém:

- `imports/`: entradas locais permitidas, identificadas por caminho relativo confinado.
- `tmp/`: transferências em construção; arquivos parciais nunca são publicados como raw íntegro.
- `raw/`: conteúdo endereçado por SHA-256, incluindo members extraídos e containers aninhados.

O transporte HTTPS valida host exato, porta, redirects e endereços públicos resolvidos, fixando os endereços na conexão. Tokens temporários não entram nos erros públicos. Importação local não permite navegar fora de `imports/` por caminho ou symlink.

O inventário conserva caminho de aninhamento, ordem da entrada, pai, hash e tamanho; diretórios/containers/entradas não selecionadas recebem motivo. Parsing recebe arquivos em disco e callbacks de registros. A descoberta usa HEAD; não recorre a GET pesado quando HEAD não é suportado.

O fluxo de duas etapas com token não é anunciado como homologado: exige canal/schema demonstrados. A Tradable Security List tem perfis CSV/JSON baseados no Security Definition Report v1.0.1, com grupos tipados e referência fornecida pelo catálogo; isso ainda não comprova compatibilidade com corpus real. O perfil CSV simplificado de fixtures continua separado. Não habilitar automaticamente um adaptador desconhecido para obter uma resposta vazia.

## Backup consistente de SQL e raw

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

Data: **06/11/2026**. Os valores abaixo pertencem a uma fixture gerada localmente, nunca a um arquivo B3. A fixture imita o formato fixo de 245 bytes para exercitar a aplicação; não é evidência de schema oficial, arquivo válido publicado pela B3, pregão real, completude ou autorização de uso.

### Ambiente e fixture

- Imagens release `0.0.1` de API, worker e migrate, construídas localmente a partir do código desta entrega.
- Docker Engine 29.8.2, arquitetura `aarch64`, 2 CPUs e aproximadamente 8 GiB reportados pelo runtime.
- PostgreSQL **16.15**, instância Compose dedicada e descartável via [`docker-compose.benchmark.yml`](../docker-compose.benchmark.yml), sem acesso à base operacional.
- Batch: 500 registros / 4 MiB; pool API padrão de 8 conexões; lease de worker de 120 s.
- Um arquivo COTAHIST-shaped, 20.000 detalhes, data única `2025-01-02`, um member, 4.920.492 bytes, SHA-256 `a001f2f338f8393aeb3b9eeb7620e3130073ecc692a5b4313b9740e192935c67`. Um canal e toda elegibilidade foram semeados com a anotação `SYNTHETIC BENCHMARK ONLY` somente neste banco temporário.
- O dataset publicado continha 20.000 fatos, sem rejeições, todos sem cadastro (estado `unresolved`). Essa configuração confirma que a incerteza de identidade não descarta o preço.

### Resultados observados

| Fase/medida | Resultado | Como interpretar |
| --- | ---: | --- |
| Importação/hash local | 86 ms | Não mede download HTTPS nem rede B3 |
| Inventário de um member simples | 40 ms | Sem ZIP ou expansão |
| Parsing + staging | 21,581 s | 20.000 registros válidos; maior parte do tempo da execução |
| Tentativa worker completa até candidata pronta | 21,802 s | Aproximadamente 917 registros/s; não inclui espera em fila |
| Publicação explícita | 43 ms | Uma promoção de revisão com 20.000 fatos |
| Pico RSS do worker, amostrado | 5,93 MiB | 12 amostras entre início e encerramento; não é profiling de alocação |
| Pico CPU do worker, amostrado | 8,5% | Maior amostra do Docker stats, não média de CPU |
| Aumento de WAL PostgreSQL | 45.880.583 bytes | `pg_stat_wal` antes/depois na instância dedicada durante a ingestão |
| Banco após ingestão/publicação | 37.895.191 bytes | Tamanho total do database, incluindo schema e índices |
| Raw endereçado por conteúdo | 4.920.492 bytes | Um objeto raw; input importado também existe sob `imports/` |
| REST concorrente | 160 requests, 8 clientes | Cada resposta limitada a 100; total 16.000 linhas retornadas |
| Latência REST | p50 663 ms; p95 888 ms | Medida no segundo ensaio com o script abaixo |
| Throughput REST | 11,64 requests/s | Um endpoint e filtro fixo, sem cache externo |
| API RSS / CPU, amostrados | 8,79 MiB / 6,2% | 7 amostras durante a carga concorrente |
| Clientes PostgreSQL máximos | 9 | Inclui uma conexão `psql` observadora; o pool da API está limitado a 8 |
| Backup SQL+raw | 1,902 s | Snapshot consistente com um raw e uma revisão ativa |
| Restore | 2,387 s | Banco novo; 20.000 linhas de referência verificadas |

O restore preservou a revisão ativa `b312beb8-97d8-4dff-9e6d-294d8c44f0e4` e digest `fc813955cada7569141c682bd5a9d92eb19b1569b23cf04046cfa1ad11ead50f`. Depois do restore, a API iniciou no banco novo e respondeu health 200 e uma página de 100 fatos com proveniência.

### Reprodução

Use sempre um projeto Compose exclusivo. Este arquivo não publica a porta PostgreSQL no host; o volume é local e descartável.

```bash
COMPOSE_PROJECT_NAME=tucano-b3-benchmark docker compose -f docker-compose.benchmark.yml up -d b3_postgres
docker compose -f docker-compose.benchmark.yml -p tucano-b3-benchmark ps
```

Aplique as migrations na base `tucano_b3_benchmark`. Gere a fixture e compare seu checksum:

```bash
mkdir -p /tmp/tucano-b3-benchmark/imports/bench
python3 scripts/generate_synthetic_cotahist.py \
  --records 20000 \
  --trade-date 2025-01-02 \
  --output /tmp/tucano-b3-benchmark/imports/bench/cotahist.txt \
  --command-file /tmp/tucano-b3-benchmark/ingest.json
shasum -a 256 /tmp/tucano-b3-benchmark/imports/bench/cotahist.txt
```

Cadastre no banco temporário o channel local `synthetic-benchmark-2025`, tipo `COTAHIST_YEARLY`, perfil `cotahist_latin1`, período 2025 e caminho `bench/cotahist.txt`. Para esse canal isolado, crie evidência de teste para os propósitos `acquisition`, `raw_retention`, `processing`, `rest`, `mcp` e `third_party_delivery`, sempre com referência literal `SYNTHETIC BENCHMARK ONLY`. **Nunca copie essa elegibilidade para uma base ou canal real.** Monte o mesmo diretório absoluto `/tmp/tucano-b3-benchmark` em API/worker e defina `B3_RAW_DATA_DIR` para esse caminho para que a ferramenta de backup consiga verificar os localizadores raw no host.

Submeta `ingest.json` através da CLI/API, execute `worker --once --lane maintenance`, publique a revisão por um job administrativo e inicie a API no mesmo banco. Os scripts versionados reproduzem o corpo da fixture e a carga de consulta:

```bash
python3 scripts/benchmark_api_reads.py \
  --base-url http://127.0.0.1:18012 \
  --token <read-token-temporario> \
  --requests 160 --concurrency 8 --limit 100
```

Para repetir o snapshot, defina `COMPOSE_FILE=docker-compose.benchmark.yml` e `COMPOSE_PROJECT_NAME=tucano-b3-benchmark` no ambiente ao executar `scripts/release_snapshot.py backup/restore`; assim o cliente Docker acessa o PostgreSQL isolado correto. Encerre apenas esse projeto e seu volume descartável ao terminar.

### Limites dos resultados

Os números caracterizam um arquivo sintético de 4,9 MB, uma família, um member e um pregão; não representam aquisição anual oficial, várias sobreposições, identidade cadastrada, backfill 2026, armazenamento de longo prazo, concorrência de ingestão com API ou escala de produção. RSS/CPU foram obtidos por amostragem de containers. A observação de WAL é específica da instância dedicada. **Nenhum resultado fixa os budgets de produção**; a qualificação com corpus autorizado e hardware-alvo segue pendente.
