---
title: Proveniência e auditoria
---

# Proveniência, observação e auditoria

Cada fato consultável mantém uma referência para explicar de qual artefato e localização veio. A resposta de mercado inclui proveniência até o registro original, não apenas o nome da família.

## Localizadores

| Campo | O que identifica |
| --- | --- |
| source_file_id | Registro operacional do arquivo preservado |
| source_file_type | Família ou edição lógica do artefato |
| sha256 | Hash dos bytes do artefato bruto |
| source_member_id | Entrada inventariada dentro do artefato |
| source_member_name | Nome/caminho lógico do member |
| record_index | Índice do registro original dentro do member |

O nome do member é lógico. A API não expõe caminho privado do filesystem, URL tokenizada nem credenciais.

## Hashes e valores repetidos

Hash de negócio é determinístico e independe de metadados operacionais. Uma reobservação idêntica não deve criar mudança de negócio ou evento de auditoria falso. Contadores de inserção, alteração, inalterado, duplicata e quarentena registram resultados efetivamente persistidos.

## Mudança de identidade

Uma resolução tardia pode acrescentar associação quando novos registros cadastrais fornecem evidência. Essa atualização preserva valor de mercado e origem original. Estados unresolved e ambiguous permanecem expressos quando não há base suficiente.

## Datas

Data de pregão é data de mercado. Horários de aquisição, processamento, mudança de negócio, observação e publicação são instantes diferentes e usam UTC no armazenamento/contrato. Não use o timestamp de download como precedência editorial de uma fonte B3.
