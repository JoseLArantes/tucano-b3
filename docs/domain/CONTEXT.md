# Contexto: histórico de mercado B3

Este contexto cobre a proveniência, a aquisição, o armazenamento e a exposição de dados históricos de mercado da B3.

## Vocabulário

- **Calendário de mercado**: calendário anual publicado pela B3 que registra feriados, suspensões de negociação e exceções de funcionamento. Um calendário é identificado pelo ano civil e mantém as URLs que fundamentam seus eventos.
- **Evento de calendário**: data e descrição do feriado ou exceção de funcionamento, com um estado de sessão: `closed` (sem negociação), `special_hours` (horário diferente do padrão) ou `open` (funcionamento normal apesar da data comemorativa).
- **Horário de negociação**: janela geral de abertura e fechamento publicada para uma modalidade. Não substitui exceções de data informadas no calendário anual.
- **Modalidade Trader**: modalidade `Tela`, `RFQ` ou `Voice` cujos horários constam na página de negociação do Trader da B3.
- **Fonte oficial**: página ou documento publicado pela B3 usado para preencher um calendário ou horário. A URL é preservada no contrato servido.
- **Ledger**: banco SQLite embarcado que mantém os calendários anuais e os registros operacionais do serviço.

## Regras do contexto

- Cada calendário cobre um único ano e seus eventos precisam ter datas ISO válidas desse mesmo ano.
- O calendário 2026 é semeado uma única vez por ledger. Operações posteriores de criação, substituição e exclusão são persistidas no SQLite.
- Horários são apresentados no fuso `America/Sao_Paulo`; horários especiais pertencem ao evento da data e não alteram automaticamente o horário geral da modalidade.
- A API preserva as fontes fornecidas e não infere feriados nem horários que não estejam descritos nas fontes.
