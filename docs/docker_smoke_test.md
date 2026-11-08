# Smoke test da stack Docker Compose

## Objetivo

Confirmar que `docker compose up -d` aplica migrations antes dos processos de runtime e inicia PostgreSQL, API e worker sem depender de fontes externas habilitadas.

## Pré-requisitos

- Docker Compose disponível.
- `.env` criado a partir de `.env.example`.
- Tokens Bearer de leitura e administração distintos e não vazios.
- Portas locais `8010` (API) e `5433` (PostgreSQL) disponíveis.

## Passos

1. Validar o arquivo de Compose e iniciar a stack pelo comando padrão:

   ```bash
   docker compose config --quiet
   docker compose up -d
   ```

   O Compose aguarda PostgreSQL ficar saudável, executa `b3_migrate` como tarefa única e só então libera API e worker.

2. Conferir o estado de todos os containers, incluindo o serviço de migration concluído:

   ```bash
   docker compose ps --all
   ```

   Resultado esperado: `b3_postgres` saudável, `b3_api` saudável, `b3_worker` em execução e `b3_migrate` encerrado com código zero.

3. Confirmar liveness HTTP e conectividade com PostgreSQL:

   ```bash
   curl --fail http://localhost:8010/healthz
   curl --fail http://localhost:8010/api/v1/health
   ```

   `/healthz` confirma que o processo HTTP respondeu. `/api/v1/health` também consulta PostgreSQL. As duas rotas são exceções públicas documentadas.

4. Verificar inicialização do worker sem iniciar aquisição externa:

   ```bash
   docker compose logs --tail=100 b3_worker
   ```

   O worker não publica endpoint HTTP de saúde. Confira os logs de startup e o estado `Up`; `B3_DISCOVERY_ENABLED` permanece `false` por padrão.

## Falhas

- Se `b3_migrate` terminar com erro, Compose não inicia API nem worker. Corrija o erro de migration ou a credencial `B3_MIGRATION_DATABASE_URL` antes de tentar novamente.
- Se a API encerrar com erro de credencial, defina tokens Bearer distintos e não vazios em `.env`.
- Se `/api/v1/health` falhar com `503`, verifique o estado e os logs do PostgreSQL.
- Não use `docker compose down -v` para corrigir inicialização; isso apaga volumes persistentes.
