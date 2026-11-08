---
title: Autenticação
---

# Autenticação e autorização

As rotas protegidas usam o cabeçalho HTTP Authorization com esquema Bearer. O serviço exige credenciais de leitura e administração distintas e não vazias; quando a configuração não é válida, uma rota protegida retorna 503 com código AUTH_NOT_CONFIGURED.

    Authorization: Bearer SEU_TOKEN

## Papéis

| Credencial | Uso |
| --- | --- |
| Token de leitura | Consultas COTAHIST e cadastrais, capacidades e diagnósticos de cobertura |
| Token administrativo | Operações de escrita e inspeções operacionais; também pode consultar rotas de leitura |
| Sem token | Somente exceções públicas explícitas de saúde e documentação |

As variáveis atuais são TUCANO_B3_READ_TOKEN e TUCANO_B3_ADMIN_TOKEN. Configure valores distintos. A antiga TUCANO_B3_TOKEN não substitui essas duas credenciais no contrato inicial. O Swagger UI não lê o arquivo .env do servidor: o botão “Authorize” guarda somente o valor Bearer que você inserir.

GET /healthz, GET /api/v1/health, GET /openapi.json, GET /docs e os assets GET sob /docs/ são exceções públicas para servir a interface Swagger. A exceção é por método e caminho GET; uma requisição de escrita para esses caminhos continua protegida. A UI não autentica chamadas automaticamente: use “Authorize” para informar o Bearer correto. Para POST e inspeções operacionais, use o valor de TUCANO_B3_ADMIN_TOKEN; o token de leitura retorna 403 ADMIN_REQUIRED nessas operações. O OpenAPI exibe essa distinção na descrição de autenticação.

## Rotas administrativas

Todo método diferente de GET exige o token administrativo. Além disso, GET em /api/v1/b3/jobs, /api/v1/b3/sync-runs, /api/v1/b3/source-files, /api/v1/b3/pending-updates, /api/v1/b3/data-revisions e seus subcaminhos também exige administração. A rota de diagnóstico de cobertura e o catálogo de capacidades aceitam o token de leitura.

## Respostas de autorização

- **401 UNAUTHORIZED**: token ausente ou inválido.
- **403 ADMIN_REQUIRED**: token de leitura usado em uma operação administrativa.
- **503 AUTH_NOT_CONFIGURED**: tokens não foram configurados corretamente no serviço.

Não envie tokens na URL, em query strings, em exemplos públicos ou em logs. Use uma credencial de leitura para aplicações consumidoras e reserve a credencial administrativa para operadores autorizados.
