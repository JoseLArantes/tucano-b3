# 07 — Homologar boletins 086 e 186 com edições e semântica próprias

Status: needs-triage

Data: 05/11/2026. Marco: G6. Dependências: plataforma 02–06 e elegibilidade própria em 01.

## Problema

Relatórios completos, simplificados, edições e prazos não podem ser fundidos por coincidência de ticker/data ou schema XML.

## Escopo

- Corpus oficial com namespaces, várias edições/members, mercado de termo e campos opcionais.
- Adaptadores, chaves e DTOs específicos para 086 e 186.
- Política de publicação e comparação de campos equivalentes, incluindo unidades e sessões.
- Cobertura por família, testes de recuperação e documentação.

## Critérios de aceite

- Nenhum fallback ou overwrite de uma família pela outra.
- `days_to_settlement` e discriminadores homologados preservam fatos distintos.
- Seleção de edição não depende da ordem do ZIP ou do download.
- Comparação distingue ausência de dado, unidade incompatível e divergência de valor.
- Histórico efetivamente disponível é medido por família; não recebe a promessa de 1986 do COTAHIST.

## Comments

- 05/11/2026 — Seções 5, 8–10, 13 e 18 do [PRD](../../../docs/b3_prd.md). Não adicionar BVBG.187 ou TradeInformationConsolidated sem decisão explícita de escopo.
