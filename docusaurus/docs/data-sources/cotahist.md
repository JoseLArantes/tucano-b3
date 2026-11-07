---
title: COTAHIST
---

# COTAHIST: histórico nominal de cotações

COTAHIST é a família de arquivos oficiais da B3 consultada pelas rotas /api/v1/b3/cotahist/quotes e /api/v1/b3/cotahist/series. Arquivos diários, mensais e anuais continuam identificáveis na proveniência.

## Interpretação dos valores

- Valores financeiros e quantidades são strings decimais, com precisão preservada.
- A moeda é a publicada no registro; não há conversão retroativa para BRL.
- FATCOT é fator de cotação, não ajuste por provento ou evento corporativo.
- PREULT é o preço do último negócio registrado no arquivo.
- Número de negócios, quantidade negociada e volume são campos distintos.
- DISMES (distribution_number) faz parte da granularidade do fato.
- Prazo a termo é um discriminador da linha.

## Leiaute e codificação

O parser valida registros por largura em bytes e preserva o artefato bruto. Há uma recuperação limitada para um detalhe de 246 bytes que contenha exatamente um caractere UTF-8 representável em ISO-8859-1 nas colunas textuais: o parser reconstrói a largura de 245 bytes e aceita a linha somente se todos os campos passarem pela validação. A normalização fica contada nos logs operacionais. Outras larguras divergentes não são ajustadas automaticamente e seguem para quarentena ou falha estrutural.

> **Aviso obrigatório:** os preços são nominais brutos, na moeda e forma de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos. O serviço não preenche pregões ausentes nem apresenta série derivada como publicação oficial B3.

## Cobertura

A faixa de datas observadas não prova cobertura contínua. A resposta informa cobertura para os filtros e revisão selecionados, contagem de rejeições e registros sem associação de identidade. Limites do corpus dependem dos arquivos efetivamente adquiridos e publicados neste ambiente.

## Desambiguação

Uma mesma combinação de ticker e pregão pode ter múltiplas linhas por mercado, BDI, distribuição ou prazo. A consulta de fatos pode listá-las conforme os filtros. A rota de série exige uma linha inequívoca e retorna 409 quando permanece mais de uma linha incompatível.

## Fonte oficial

O layout de COTAHIST e suas definições oficiais são a referência para interpretar os códigos. Consulte o [guia B3 do repositório](https://github.com/JoseLArantes/b3-tucano/blob/main/docs/B3.md) e [pesquisa de fontes](https://github.com/JoseLArantes/b3-tucano/blob/main/docs/RESEARCH.md) para links e limitações documentadas.
