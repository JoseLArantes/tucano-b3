---
title: Cobertura e atualização de ações
sidebar_position: 3
---

# Cobertura e atualização de ações

## Dataset verificado

O parser lê os campos posicionais COTAHIST para mercado, BDI, ticker, especificação, moeda, `PREULT`, `FATCOT` e ISIN. A API de ações e units usa uma saída própria em `data/processed/equities-v1/`, com schema versionado. Ela é reconstruída com os arquivos oficiais sem depender do ledger legado que registra pregões já ingeridos.

A tabela de ações e a consulta de preços exigem mercado spot `010`, BDI `02`, BRL, ISIN com dígito verificador e especificação `ON`, `PN` ou `UNT`. A filtragem antecede a paginação. Os endpoints publicados expõem a cobertura que foi efetivamente verificada; nenhuma observação é retornada fora desse período.

## Atualização e implantação

Faça o backfill oficial após publicar a nova versão do serviço. A ingestão reprocessa todos os registros para `equities-v1` ainda que o ledger principal marque as datas como já processadas. Reinicie ou recarregue o serviço para que o DataFusion registre os novos arquivos.

Confirme `coverage.from` e `coverage.to` em `GET /api/v1/cotahist/equities` e compare amostras do mesmo pregão com as linhas oficiais COTAHIST. Valide ao menos ações ON/PN, uma unit, um ticker renomeado e um ticker reutilizado. Garanta que uma linha de mercado fracionário e um ETF não apareçam no picker.

## Interpretação dos preços

O `PREULT` do arquivo contém duas casas decimais implícitas. A cotação por ação ou unit é `PREULT / 100 / FATCOT`. O valor é nominal e não ajustado por desdobramentos, grupamentos, bonificações ou outros eventos corporativos. Para comparar retornos ou valuar posições ao longo do tempo, consumidores devem considerar os eventos corporativos separadamente.

Fonte do layout posicional: [COTAHIST — layout divulgado pela B3](https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf).

## Publicar volume financeiro exato

Reimporte os arquivos originais COTAHIST via upload, pasta ou bucket para cobrir o intervalo do piloto. Inclua o ano anterior quando a janela atravessar a virada. Volumes legados armazenados como números de ponto flutuante não permitem recuperar os centavos originais.

A ingestão lê `VOLTOT` antes do filtro legado de datas. A mesma observação repetida em arquivos anuais, mensais ou diários mantém a origem já aceita. Se todos os registros forem repetidos, a revisão também permanece. Em caso de conflito no mesmo ISIN e pregão, consulte o erro de ingestão e compare a linha recebida com a fonte já aceita. O membro em conflito não publica uma nova revisão.

O dataset é publicado em Parquet por ano, com a revisão e os caminhos ativos no ledger SQLite. Preserve ambos no backup. A consulta passa a ver a nova revisão após o recarregamento dos dados. Arquivos de gerações anteriores permanecem em disco para leitores que ainda usam a revisão anterior.

Após a carga, consulte `/turnover` para amostras ON, PN e unit, confira as datas devolvidas e compare `voltot_raw` e `source_line` com os membros originais. Use o [layout oficial](https://b3.com.br/data/files/C8/F3/08/B4/297BE410F816C9E492D828A8/SeriesHistoricas_Layout.pdf) para localizar os 18 dígitos de `VOLTOT`.
