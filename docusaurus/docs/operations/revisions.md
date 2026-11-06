---
title: Revisões, publicação e cobertura
---

# Revisões e publicação

Uma ingestão constrói uma revisão candidata. Parsing, validações, conflitos, escopo de data e todos os artefatos/members esperados são registrados antes de uma mudança de visibilidade. Uma candidata COTAHIST pode reunir vários arquivos anuais/mensais/diários selecionados para o mesmo pedido.

## Inspeção

GET /api/v1/b3/data-revisions/&#123;id&#125; mostra family, revision_kind, intervalo solicitado, arquivos reunidos, members esperados, rejeições, conflitos, registros válidos fora do corte (`out_of_scope`), timestamps e proveniência. O schema mantém a família `registry` para evolução do modelo, mas import cadastral está `deferred` e não compõe os dados entregues pelo `0.0.1`. Uma resposta de inspeção não altera a revisão.

Estados:

- draft: candidata em construção;
- ready: candidata completa e apta a publicação automática se os gates estiverem limpos, ou aguardando decisão para reconhecer issues;
- published: promovida pelo worker após import limpo ou por operação administrativa autorizada;
- invalid: contém condição que impede publicação no estado atual.

Use GET /api/v1/b3/data-revisions/&#123;id&#125;/conflicts para versões divergentes e /api/v1/b3/data-revisions/&#123;id&#125;/quarantine para linhas rejeitadas. A ordem de download não define qual versão é autoritativa. A quarentena conserva motivo e localização para uma ação de replay posterior.

## Publicar e reverter

O import limpo publica automaticamente depois dos gates de qualidade e elegibilidade. Rejeições isoladas ou conflitos não mudam a visão ativa; `succeeded_with_issues` informa que uma candidata continua não publicada, e o administrador pode submeter `publish_revision` com reconhecimento explícito quando permitido. `rollback_revision` restaura uma revisão publicada anteriormente.

`withdraw_window` cria uma revisão `withdrawal` que omite do resultado ativo as datas inclusivas pedidas. Raw, fatos, versões, ocorrências e auditoria permanecem disponíveis; rollback restaura a visão anterior e uma nova ingestão pode reimportar o período sem criar duplicatas de fatos/versões.

## Diagnóstico de cobertura

GET /api/v1/b3/diagnostics/coverage aceita um data_revision opcional. A consulta retorna o diagnóstico da revisão publicada e elegível. Campos de primeira/última observação não provam sequência completa nem tamanho total do universo. Rejeições, identidade não resolvida/ambígua e estado de calendário continuam visíveis.

## Elegibilidade

GET /api/v1/b3/capabilities retorna source_file_type, implemented, enabled e status. status é enabled, not_enabled ou deferred. enabled significa que a família tem canal e evidência de elegibilidade para REST/terceiros; não prova que haja arquivo ou cobertura. As famílias implementadas são BVBG.028, TSL e COTAHIST diário/mensal/anual. PriceReport, Simplified Price Report, IndexReport e composição de índice constam como adiados no catálogo atual.
