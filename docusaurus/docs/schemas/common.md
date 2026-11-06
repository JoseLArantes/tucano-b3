---
title: Schemas comuns
---

# Schemas comuns

O OpenAPI gerado no ambiente é a definição autoritativa de tipos, required/nullability e status. Esta página explica as partes compartilhadas usadas pelas respostas REST.

## Erro

| Campo | Tipo | Significado |
| --- | --- | --- |
| code | string | Código estável da categoria do erro |
| message | string | Mensagem segura em português |
| request_id | UUID | Identificador de correlação da ocorrência |

Erros não devem revelar SQL, tokens, hosts privados nem caminhos do filesystem.

## Proveniência de cotação

| Campo | Tipo | Significado |
| --- | --- | --- |
| source_file_id | string | ID do artefato de origem |
| source_file_type | string | Família oficial ou edição lógica |
| sha256 | string | Hash do arquivo bruto |
| source_member_id | string | ID do member inventariado |
| source_member_name | string | Nome/caminho lógico do member |
| record_index | inteiro | Índice do registro original |

## Identidade e status

identity.instrument_id pode ser nulo quando não há associação sustentada. identity.status usa resolved, unresolved ou ambiguous. Esses três estados são significativos e não devem ser colapsados em uma flag booleana.

Job status usa queued, running, succeeded, succeeded_with_issues, failed ou cancelled. Job phase é uma dimensão separada e pode ser queued, acquisition, inventory, parsing, staging, sealing, ready, publication, identity ou completed.

## Paginação

Páginas COTAHIST incluem items, total, limit, offset e data_revision. As respostas cadastrais expõem snapshots junto à paginação. Páginas operacionais retornam items, limit e offset; consulte o schema por rota para os detalhes.

Valores de data usam ISO 8601 em YYYY-MM-DD. Instantes usam UTC. Valores monetários, quantidades e volume são strings decimais.
