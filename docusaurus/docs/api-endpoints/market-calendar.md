---
title: Calendário B3 e horários de negociação
---

# Calendário B3 e horários de negociação

O serviço inicia com o calendário de negociação de 2026 publicado pela B3. Calendários anuais são armazenados no ledger SQLite e podem ser criados, substituídos ou removidos pela API. Os horários gerais do Trader são consultados separadamente e refletem a tabela publicada pela B3.

Todas as rotas exigem autenticação quando o serviço está configurado com credenciais. Para `POST`, `PUT` e `DELETE`, a API responde `503` se nenhuma credencial estiver configurada. Respostas de sucesso usam `{ data, meta }`; erros seguem o envelope `{ error }`.

## Operações

| Método | Rota | Uso |
| --- | --- | --- |
| `GET` | `/api/v1/calendar` | Lista anos disponíveis em ordem crescente. |
| `POST` | `/api/v1/calendar` | Cria um calendário anual que ainda não existe (`201`). |
| `GET` | `/api/v1/calendar/{year}` | Consulta o calendário e suas fontes. |
| `PUT` | `/api/v1/calendar/{year}` | Substitui integralmente eventos e fontes de um ano existente. |
| `DELETE` | `/api/v1/calendar/{year}` | Exclui o calendário anual persistido. |
| `GET` | `/api/v1/negotiation-hours` | Consulta os horários gerais das modalidades Trader `Tela`, `RFQ` e `Voice`. |

## Calendários anuais

Use `POST` para criar outros anos. `PUT` substitui o calendário inteiro; datas e nomes não são mesclados com a versão anterior. O ano precisa estar entre 1900 e 2200, cada evento deve pertencer a esse ano, as datas de eventos devem ser únicas e cada calendário precisa de uma a dez URLs HTTP ou HTTPS do domínio oficial `b3.com.br` ou de seus subdomínios. Os horários usam `HH:MM`. Um evento `special_hours` informa ao menos um horário; os estados `open` e `closed` não recebem horários intradiários.

```bash
curl -X POST http://localhost:8080/api/v1/calendar \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{
    "ano": 2027,
    "fontes": ["https://www.b3.com.br/pt_br/"],
    "eventos": [{
      "data": "2027-01-01",
      "evento": "Confraternização Universal",
      "situacao": "closed",
      "descricao": "Não haverá sessão de negociação.",
      "pre_abertura": null,
      "inicio_negociacao": null,
      "fim_negociacao": null
    }]
  }'

curl http://localhost:8080/api/v1/calendar/2026 \
  -H "Authorization: Bearer $TUCANO_B3_TOKEN"
```

O `DELETE` de 2026 também é persistente: a carga inicial é aplicada uma única vez por ledger e não restaura o calendário depois de removido. Crie o ano novamente com `POST` se precisar reintroduzi-lo.

O calendário 2026 inclui os dias sem negociação, a abertura especial da Quarta-feira de Cinzas (pré-abertura às 12h45 e negociação a partir das 13h) e a indicação de funcionamento normal em 9 de julho. A página oficial de [calendário de 2026 da B3](https://www.b3.com.br/pt_br/noticias/calendario-de-negociacao-da-b3-confira-o-funcionamento-da-bolsa-em-2026.htm) e a [Circular 003/2026-VNC](https://www.b3.com.br/data/files/FC/55/3B/12/7FE9B9109B5E99B9AC094EA8/OC%20003-2026-VNC%20ERRATA_CALENDARIO%20DE%20FERIADOS%20EM%202026%20E%20FUNCIONAMENTO%20DA%20B3%20EM%2018022026%20QUARTAFEIRA%20DE%20CINZAS_PT.pdf) são preservadas como fontes do calendário.

## Horários gerais do Trader

`GET /api/v1/negotiation-hours` retorna os valores da página de [horários de negociação da B3](https://www.b3.com.br/pt_br/solucoes/plataformas/interface-de-negociacao/trader/horarios-de-negociacao.htm). Os horários são apresentados em `America/Sao_Paulo`:

| Modalidade | Abertura | Fechamento |
| --- | --- | --- |
| Tela | 09:00 | 18:00 |
| RFQ | 09:00 | 18:00 |
| Voice | 08:00 | 19:15 |

Esses são horários gerais da plataforma Trader. Eles não substituem horários especiais de datas específicas no calendário anual.

Consulte `/openapi.json` para os schemas fechados, códigos de erro, autenticação e exemplos completos. O Swagger UI em `/docs` carrega o mesmo documento; a autorização das chamadas segue as regras da API. `/api/docs` permanece como alias por compatibilidade.
