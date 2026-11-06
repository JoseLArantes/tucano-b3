---
title: Revisões, publicação e cobertura
---

# Revisões e publicação

Uma ingestão constrói uma revisão candidata. Parsing, validações, conflitos e membros esperados devem ser registrados antes que uma operação administrativa publique a revisão.

## Inspeção

GET /api/v1/b3/data-revisions/&#123;id&#125; mostra family, período, status, quantidade de members esperados, rejeições, conflitos, timestamps e proveniência. family distingue cotahist de registry. Uma resposta de inspeção não altera o candidato.

Estados:

- draft: candidata em construção;
- ready: preparada para decisão de publicação;
- published: promovida explicitamente;
- invalid: contém condição que impede publicação no estado atual.

Use GET /api/v1/b3/data-revisions/&#123;id&#125;/conflicts para versões divergentes e /api/v1/b3/data-revisions/&#123;id&#125;/quarantine para linhas rejeitadas. A ordem de download não define qual versão é autoritativa. A quarentena conserva motivo e localização para uma ação de replay posterior.

## Publicar e reverter

Submeta publish_revision ou rollback_revision como job administrativo. A promoção não acontece por consultar ou construir uma candidata. A revisão anterior permanece ativa até a operação confirmada. Veja o estado e outputs do job para verificar qual recurso foi persistido.

## Diagnóstico de cobertura

GET /api/v1/b3/diagnostics/coverage aceita um data_revision opcional. A consulta retorna o diagnóstico da revisão publicada e elegível. Campos de primeira/última observação não provam sequência completa nem tamanho total do universo. Rejeições, identidade não resolvida/ambígua e estado de calendário continuam visíveis.

## Elegibilidade

GET /api/v1/b3/capabilities retorna source_file_type, implemented, enabled e status. status é enabled, not_enabled ou deferred. enabled significa que a família tem canal e evidência de elegibilidade para REST/terceiros; não prova que haja arquivo ou cobertura. As famílias implementadas são BVBG.028, TSL e COTAHIST diário/mensal/anual. PriceReport, Simplified Price Report, IndexReport e composição de índice constam como adiados no catálogo atual.
