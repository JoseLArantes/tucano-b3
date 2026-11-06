---
title: Introdução
sidebar_position: 1
---

# Tucano B3

O **Tucano B3** preserva e consulta informações oficiais da B3 com histórico de origem, valores decimais precisos e identidade de instrumento contextualizada no tempo. Ele organiza arquivos públicos em contratos REST e em ferramentas analíticas MCP somente leitura.

## O que o serviço responde

O objetivo é permitir consultas que mantenham o contexto necessário para interpretar um registro:

- qual publicação e família de arquivo sustentam o dado;
- qual era o mercado, a moeda, o fator de cotação e os demais qualificadores originais;
- qual instrumento pode ser associado ao ticker naquela data;
- em qual arquivo, member e registro a informação foi encontrada;
- qual revisão foi consultada e quais limites de cobertura ou incerteza permanecem.

O serviço mantém separados fatos de negociação, fotografias cadastrais e estado operacional. Um ticker sozinho não representa uma identidade permanente. Datas cadastrais não são estendidas a períodos sem evidência e conflitos não são resolvidos escolhendo uma linha arbitrária.

## Estado documentado

A API REST inicial oferece consulta COTAHIST, evidências cadastrais publicadas e inspeção/submissão de operações duráveis. A API inclui também endpoints operacionais para arquivos, members, pendências, revisões, conflitos, quarentena, capacidades, diagnósticos e execuções. As fontes e operações permitidas dependem de registro e elegibilidade configurados no banco; a presença de uma rota não significa que haja dados ou autorização para distribuição externa.

O escopo do produto inclui outras publicações oficiais B3, como BVBG.086, BVBG.186, BVBG.087 e composições de índices. Consulte o guia de fontes e o endpoint de capacidades para separar escopo planejado, implementação e disponibilidade real em um ambiente.

> **Aviso sobre COTAHIST:** preços históricos nominais brutos na moeda e na forma de cotação da época, sem ajustes retroativos por inflação ou eventos corporativos. PREULT é o último negócio publicado e não deve ser renomeado como um preço de fechamento genérico.

## Para quem é esta documentação

- **Consumidores da API**: credenciais, filtros, paginação, schemas, revisão fixada e respostas de erro.
- **Operadores**: descoberta, jobs, tentativas, cancelamento, revisões, arquivos, members e quarentena.
- **Pesquisadores**: semântica das fontes, identidade temporal, limitações de cobertura e proveniência.
- **Contribuidores**: organização da documentação e sua relação com PRD, código e OpenAPI.

## Navegação recomendada

1. [Primeiros passos](getting-started/installation.md): configure o serviço e obtenha a URL local.
2. [Autenticação](getting-started/authentication.md): entenda os papéis de leitura e administração.
3. [Consulta COTAHIST](getting-started/quickstart.md): faça a primeira chamada paginada.
4. [Padrões comuns da API](api-endpoints/common-patterns.md): consulte paginação, precisão, revisão e erros.
5. [Operação](operations/overview.md): acompanhe a fila administrativa e a publicação de dados.

## Três referências diferentes

| Recurso | Conteúdo |
| --- | --- |
| Site Docusaurus | Guias detalhados para consumidores, operadores e pesquisa |
| /openapi.json | Contrato JSON gerado a partir das operações e schemas Utoipa |
| /docs da API | Página HTML mínima do serviço com link para /openapi.json |

Para mudanças de contrato, confira o JSON produzido pelo binário e a página Docusaurus correspondente. O PRD explica o comportamento-alvo; páginas de referência deste site descrevem o comportamento disponível.
