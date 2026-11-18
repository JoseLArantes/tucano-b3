---
title: Ingestão de índices
---

# Ingestão de índices da B3

O serviço baixa o CSV anual da página de evolução diária do IBOVESPA, IDIV e IFIX e grava os pregões em Parquet. Na primeira inicialização, processa primeiro o ano corrente e depois tenta preencher os anos disponíveis desde 1998 para IBOV, 2005 para IDIV e 2010 para IFIX. O worker agenda a atualização do ano corrente diariamente às 02:00 no fuso `America/Sao_Paulo`.

O status operacional está em `GET /api/v1/ingest/indices/status`. `last_successful_ingestion` informa quando uma partição foi publicada pela última vez. `latest_trading_date` informa a observação de pregão mais recente que está carregada.

Para iniciar manualmente uma ingestão, envie `POST /api/v1/ingest/indices` com uma credencial válida. Sem corpo, solicita os três índices no ano corrente. A seleção aceita os códigos `ibov`, `idiv` e `ifix`; `ano_inicio` e `ano_fim` definem um intervalo inclusivo.

```json
{
  "indices": ["ibov", "idiv"],
  "ano_inicio": 2024,
  "ano_fim": 2025
}
```

O endpoint responde `202` com o identificador da execução. Consulte `GET /api/v1/ingest/indices/{id}` para acompanhar o estado e os resultados por índice e ano. Uma execução manual concorrente recebe `409`.

O worker mantém snapshots anuais no diretório de dados e registra no SQLite o snapshot ativo, o hash da origem, a cobertura de datas e os resultados dos trabalhos. Arquivos B3 vazios ou malformados não são publicados. As falhas ficam visíveis no estado do trabalho e partições não concluídas são tentadas novamente pelo agendador.
