# Docker Smoke Test (Task 3)

## Objetivo
Validar operacao local da stack completa para API e worker (com scheduler embutido).

## Passos
1. Subir a stack:
   - `docker compose up --build -d`
2. Conferir status:
   - `docker compose ps`
3. Validar saude da API:
   - `curl -H "Authorization: Bearer dev-token" http://localhost:8001/api/v1/health`
4. Validar saude do worker/broker:
   - `curl -H "Authorization: Bearer dev-token" http://localhost:8001/api/v1/b3/worker/health`
5. Executar testes unitarios no container:
   - `docker compose run --rm api pytest -q`

## Resultado esperado
- `api`, `worker`, `postgres`, `redis` ativos.
- Endpoints de saude retornando `{"status":"ok"}`.
- Testes passando.
