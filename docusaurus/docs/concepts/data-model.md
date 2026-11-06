---
title: Modelo de dados
---

# Modelo de dados

O modelo separa registros de mercado, evidências cadastrais, artefatos de origem e revisões publicáveis. Essa separação mantém preservados os significados e a linhagem diferentes das publicações B3.

## Entidades principais

| Entidade | Significado |
| --- | --- |
| Instrumento B3 | Raiz canônica de identidade quando a evidência permite associar um registro |
| Fato COTAHIST | Ocorrência de negociação publicada com mercado, data, distribuição e prazo |
| Evidência cadastral | Observação em uma fotografia oficial, ligada à fonte e ao snapshot que a sustenta |
| Arquivo de origem | Artefato imutável recebido, com hash, tamanho, tipo e canal de aquisição |
| Member | Entrada inventariada dentro do arquivo ou container, inclusive quando excluída da interpretação |
| Revisão | Conjunto controlado de fatos ou fotografia cadastral com estado explícito de preparação e publicação |
| Job | Intenção administrativa durável que compartilha estado entre API, CLI e worker |

## Chaves de fatos de mercado

Ticker, tipo de mercado, BDI, distribuição do papel e prazo são discriminadores importantes. DISMES corresponde à distribuição do papel e faz parte da granularidade do registro. O prazo diferencia negociações a termo. O código do mercado é texto e deve manter zeros à esquerda.

Fatos com identidade desconhecida ainda podem ser preservados. A relação com um instrumento e o valor de mercado têm ciclos de vida próprios: reconstruir a identidade não deve alterar preço, quantidade, volume ou proveniência.

## Valores e tempo

Preços, quantidades e volume são strings decimais na resposta. A API não transforma esses valores para ponto flutuante, moeda corrente ou base ajustada. Datas de pregão são datas de mercado; timestamps de ingestão, observação e publicação são UTC.

## Interfaces

- REST oferece consultas e comandos administrativos duráveis.
- MCP oferece ferramentas analíticas somente leitura usando os mesmos serviços de consulta.
- O worker executa jobs persistidos no PostgreSQL e registra os estados efetivamente confirmados.

As interfaces não mantêm regras de negócio paralelas. A API e o MCP retornam contratos adequados a cada transporte, mas respeitam as mesmas regras de precisão, identidade, proveniência e elegibilidade.
