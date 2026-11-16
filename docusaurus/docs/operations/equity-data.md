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
