# Proventos

O serviço oferece CRUD autenticado para `Dividendo`, `JCP` e `Rendimento`, persistido no ledger SQLite. Endpoints disponíveis:

- `GET /api/v1/proventos` — filtros por ticker, tipo e intervalo inclusivo de data de pagamento, com paginação padrão de 50 e máximo de 1000.
- `POST /api/v1/proventos` — cria registro (`201`).
- `GET /api/v1/proventos/{id}` — consulta registro.
- `PUT /api/v1/proventos/{id}` — substitui campos do registro.
- `DELETE /api/v1/proventos/{id}` — remove registro.

Sucessos usam o envelope `{ data, meta }`. Os campos são `ticker`, `tipo`, `valor`, `data_com`, `data_pagamento` e `obs`. Datas são ISO (`AAAA-MM-DD`) ou nulas. `valor` é enviado como decimal textual positivo em reais, com até seis casas, e sempre retorna formatado com seis casas. `obs` deve conservar fonte e observações (até 255 caracteres). Não há deduplicação automática; cada registro tem ID estável para permitir a gestão explícita.

Um ledger novo recebe uma única carga de 34 registros de novembro de 2026. A carga não é obtida nem atualizada automaticamente. Os registros continuam editáveis e removíveis pelo CRUD; uma exclusão não é revertida depois de reiniciar o serviço.

Os registros não constituem uma série oficial da B3. O campo `obs` guarda a proveniência fornecida com cada registro, sem validação automática pela API.

Consulte a referência no `openapi.json` e os guias Docusaurus de [endpoints](../docusaurus/docs/api-endpoints/proventos.md) e [schemas](../docusaurus/docs/schemas/proventos.md).
