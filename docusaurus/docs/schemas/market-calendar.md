---
title: Schema do calendário B3
---

# Schema do calendário B3

## Calendário anual

As rotas de calendário retornam `data` e `meta`. O objeto em `data` contém:

| Campo | Tipo | Significado |
| --- | --- | --- |
| `ano` | inteiro | Ano civil coberto pelo calendário. |
| `fontes` | lista de strings | URLs HTTP ou HTTPS em `b3.com.br` ou subdomínio que fundamentam o calendário. |
| `eventos` | lista de eventos | Datas em ordem crescente, sem duplicatas. |
| `atualizado_em` | string ISO 8601 | Instante UTC da última criação ou substituição. |

Cada evento contém `data` (`AAAA-MM-DD`), `evento`, `situacao`, `descricao`, `pre_abertura`, `inicio_negociacao` e `fim_negociacao`. Campos opcionais de descrição e horário podem ser `null`.

`situacao` aceita:

- `closed`: não há sessão de negociação.
- `special_hours`: há horário excepcional; informe ao menos um dos horários do evento.
- `open`: funcionamento normal, mesmo em uma data comemorativa.

Horários usam `HH:MM` no fuso local da B3. O início deve anteceder o encerramento quando ambos são informados. A API rejeita datas fora do ano do calendário, fontes fora do domínio oficial B3, datas duplicadas e nomes de evento vazios.

## Horários de negociação

`GET /api/v1/negotiation-hours` retorna:

- `fuso_horario`: `America/Sao_Paulo`.
- `fonte`: página da B3 usada como referência.
- `modalidades`: lista com `modalidade`, `abertura` e `fechamento`.

As três modalidades publicadas são Tela (09:00–18:00), RFQ (09:00–18:00) e Voice (08:00–19:15). Estes valores são os horários gerais da página do Trader e não modelam exceções de calendário.

O schema OpenAPI em `/openapi.json` é a referência legível por máquina.
