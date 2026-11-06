# 08 — Entregar IndexReport e carteiras com história e vigência explícitas

Status: needs-info

Data: 05/11/2026. Marco: G7. Dependências: plataforma 02–06 e elegibilidade própria em 01.

## Problema

Valores de índice, IOPV, referência de BDR, composição efetiva e prévia têm significados diferentes. Carteira atual não responde consultas históricas.

## Escopo

- BVBG.087 por bloco e unidade, com catálogos e corpus homologados.
- Fontes oficiais de composição, fotografia, referência de peso, publicação, vigência e estágio.
- Fatos preservados mesmo sem vínculo, resolução temporal e qualidade.
- Pesquisa separada de estatísticas históricas de índices como possível expansão do catálogo permitido.

## Critérios de aceite

- IOPV e referências de BDR não aparecem como fechamento de índice.
- Prévia não substitui carteira efetiva; peso diário não é propagado para toda a vigência.
- Consulta sem fotografia histórica sustentada retorna ausência/limitação explícita.
- Datas iniciais divergentes entre páginas/idiomas não viram promessa de cobertura.
- REST/MCP, OpenAPI e documentação refletem os mesmos tipos e limites.
- Nenhuma fonte fora da lista atual é implementada sem decisão registrada e atualização de escopo.

## Prontidão para execução

Aguardar plataforma comum 02–06 e elegibilidade/corpus da família em 01. Expansões de fonte continuam sujeitas a decisão explícita de escopo. O status deve mudar para `ready-for-agent` somente após registrar a evidência que libera a entrega completa.

## Comments

- 05/11/2026 — Seções 5, 8, 13, 15–16 e 18 do [PRD](../b3_prd.md), com evidências [R16–R17](../RESEARCH.md#r16).

- 06/11/2026 — Issue incorporada ao backlog em `docs/issues`; dependências e prontidão revistas, sem declarar pré-requisitos concluídos.
