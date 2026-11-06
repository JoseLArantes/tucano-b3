---
title: Códigos e recuperação de erros
---

# Códigos de erro e recuperação

O schema REST de erro é code, message e request_id. Use code para classificar e request_id para correlacionar com logs autorizados. A mensagem não contém SQL nem credencial.

## Respostas REST

| HTTP | Quando aparece | Próximo passo |
| --- | --- | --- |
| 400 | Query, paginação, ID, intervalo ou JSON inválido | Corrija formato e filtros; consulte constraints do OpenAPI |
| 401 | Bearer ausente ou inválido | Envie token válido no cabeçalho Authorization |
| 403 | A rota requer administração ou a fonte não é elegível para a finalidade | Verifique o papel e o catálogo de capacidades/elegibilidade |
| 404 | Recurso, ID ou revisão não existe | Confirme UUID e ambiente |
| 409 | Série ambígua, chave idempotente incompatível, escopo concorrente ou estado não permite ação | Desambigue filtros ou inspecione o estado do job |
| 500 | Falha inesperada de consulta ou persistência | Correlacione request_id e não repita comandos cegamente |
| 503 | Dependência, configuração de auth ou revisão indisponível | Verifique saúde e estado operacional antes de retentar |

## Submissões administrativas

POST de job usa 202 com URL de acompanhamento. Repita o GET de status; não reenvie a operação como se 202 significasse falha. Se houver timeout no cliente, preserve a idempotency_key para resolver se o mesmo comando já foi aceito.

Falhas de aquisição e interpretação são estados do job/arquivo e rejeições localizadas, não erros HTTP vazios para uma consulta de mercado.

## MCP

tools/call retorna isError e conteúdo textual conforme o erro da consulta. O transporte continua somente leitura. Consulte o log do processo com cuidado; não grave argumentos sensíveis.
