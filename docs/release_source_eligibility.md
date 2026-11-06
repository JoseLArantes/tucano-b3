# Elegibilidade de fontes para o lançamento 0.0.1

Status: needs-info

Verificação: 06/11/2026. Finalidade pretendida: REST autenticado e MCP analítico com consumo por terceiros, sem custo obrigatório de dados.

## Evidência consultada

Os [Termos de Uso oficiais da B3](https://www.b3.com.br/pt_br/termos-de-uso-e-protecao-de-dados/termos-de-uso/) foram consultados nesta data. As seções de propriedade intelectual e difusão condicionam modalidades de uso e distribuição à autorização. Essa página não comprova que os canais e finalidades do lançamento têm autorização gratuita específica.

Essa evidência exige esclarecimento por canal/finalidade; não constitui conclusão de que todos os arquivos possuem condições idênticas. Não foi enviada comunicação externa nem contratado acesso.

## Matriz inicial

| Fonte/canal | Aquisição | Raw/retenção | Processamento | REST | MCP | Terceiros | Custo zero comprovado |
| --- | --- | --- | --- | --- | --- | --- | --- |
| COTAHIST anual 2025 | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | Não |
| COTAHIST acumulado 2026 | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | Não |
| COTAHIST diário/mensal | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | Não |
| BVBG.028 | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | Não |
| Tradable Security List | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | Não |
| InstrumentsConsolidated | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | unreviewed | Não |

## Evidência necessária para encerrar W1

### Complemento de pesquisa técnica e de condições em 06/11/2026

| Referência primária | O que demonstra | O que não comprova |
| --- | --- | --- |
| [Hub de dados públicos B3](https://www.b3.com.br/main.jsp?doui_processActionId=setLocaleProcessAction&locale=pt_BR&lumA=1&lumII=8AE490CA9A5E21B5019AEBE12A9C42A6&lumPageId=8AE490CA9A5E21B5019AEBE12A684286) | O aviso da página restringe coleta automatizada, inclusive por IA, e remete aos termos de uso | Não concede permissão de automação ou redistribuição para o projeto; não se deduz que todo canal oficial tenha condições idênticas |
| [Política comercial e contratos](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/market-data/distribuidores/politica-comercial-e-contratos/) | Há versões distintas antes/depois de 01/11/2026; a verificação deve usar a vigência correta | A existência de arquivo público ou política comercial não demonstra licença gratuita aplicável a cada canal |
| [Política de Consumo v1.1](https://www.b3.com.br/data/files/0D/97/1F/B7/4CCFF91094DCD8F9AC094EA8/Politica%20de%20Consumo%20de%20Market%20Data%20B3%20V1.1.pdf) | Documento com vigência iniciada em 01/11/2026 e modalidades de licenciamento | Não foi identificada nesta leitura uma liberação específica e gratuita da API externa pretendida para o corpus 2025–2026 |
| [Layout COTAHIST, revisão 2](https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf) | Estrutura de 245 bytes; a contagem no trailer inclui header e trailer | Documento de formato não é autorização de aquisição/uso; nenhum arquivo de mercado foi adquirido nesta pesquisa |
| [Security Definition Report v1.0.1](https://www.b3.com.br/data/files/09/37/C5/F8/CDAB29106EEC8429AC094EA8/Security%20Definition%20Report-v1.0.1-enUS.pdf) | Perfil CSV/JSON da Tradable Security List, identificadores qualificados e grupos repetidos; referência pertence à publicação | Não comprova retenção histórica disponível nem homologação do parser com arquivo de produção |

A classificação permanece **needs-info** por fonte/canal/finalidade: há restrições documentadas no canal do hub, mas não se concluiu impossibilidade de toda modalidade gratuita. Também não se deduz autorização de outros sites, ferramentas ou fornecedores que utilizem os mesmos dados. Não houve envio de mensagens à B3, contratação ou download de corpus.

Registrar documento/resposta aplicável, canal exato, finalidade, vigência, gratuidade, restrições de automação/armazenamento/entrega e data da revisão. Download manual permitido não libera automaticamente coleta automatizada ou distribuição por API.

O corpus congelado deve identificar arquivo/publicação, origem, SHA-256, tamanho, schema, encoding, período, membros, regras de retenção/uso em teste e calendário do corte. Nenhum arquivo oficial 2025–2026 foi adquirido nesta entrega; não há manifesto de corpus real nem cutoff homologado.

Enquanto isso, fixtures sintéticas permanecem explicitamente sintéticas. O banco não recebe registros `eligible` por seed; falta de registro nega elegibilidade no runtime. A execução verifica aquisição, retenção raw e processamento antes de baixar ou importar; replay/reparse exige retenção e processamento; publicação exige processamento, retenção e as finalidades de consumo; REST e MCP verificam a finalidade do transporte e `third_party_delivery` sobre as fontes que sustentam os resultados. Os testes cobrem recusa por ausência de evidência e por finalidade restrita. Isso comprova o controle técnico fail-closed, não concede permissão da B3 nem altera o estado `needs-info` desta matriz.

## Comments

- 06/11/2026 — Registro aberto para W1. Elegibilidade externa e custo zero continuam pendentes; publicação externa 0.0.1 não liberada.
