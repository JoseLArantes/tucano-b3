# Elegibilidade de fontes por finalidade

Status: needs-info para ativação de dados reais; controle técnico fail-closed implementado.

Verificação registrada: 06/11/2026. Esta matriz governa aquisição, retenção, processamento e entrega de dados por canal. Ela **não** exige corpus real carregado para concluir o software 0.0.1. Consulte [plano 0.0.1](0.0.1-plan.md) e [PRD](b3_prd.md).

## Evidências consultadas

- Os [Termos de Uso oficiais B3](https://www.b3.com.br/pt_br/termos-de-uso-e-protecao-de-dados/termos-de-uso/) condicionam modalidades de uso e difusão a suas regras e autorizações. A página não comprova uma autorização gratuita específica para cada canal/finalidade do projeto.
- A página B3 de [Cotações históricas](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/market-data/historico/mercado-a-vista/cotacoes-historicas/) informa o formato de distribuição e contém aviso contra coleta automatizada, inclusive por ferramentas de IA. Não automatizar a coleta deste canal enquanto a restrição aplicável não estiver esclarecida/autorizada. A regra observada nesta página não é generalizada para todos os canais B3.
- O [leiaute oficial COTAHIST](https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf) define a data do pregão em cada registro de detalhe e uma data de geração no header. O filtro de importação usa a data de negócio do registro; a data do arquivo só auxilia a seleção/auditoria da publicação.
- As especificações de [Tradable Security List](https://www.b3.com.br/data/files/09/37/C5/F8/CDAB29106EEC8429AC094EA8/Security%20Definition%20Report-v1.0.1-enUS.pdf) e demais layouts definem estrutura/semântica, não licença automática de retenção ou redistribuição.

Essas referências dão contexto técnico e registram restrições visíveis. Não foi enviada comunicação à B3, contratada licença, obtido arquivo de produção ou concluída avaliação jurídica. O status de cada finalidade permanece não confirmado quando não há evidência específica.

## Matriz de ativação

| Família / canal candidato | Origem oficial e formato | Aquisição automática | Retenção raw | Processamento | REST/MCP | Entrega a terceiros | Estado atual |
| --- | --- | --- | --- | --- | --- | --- | --- |
| COTAHIST anual | Arquivo histórico B3 ZIP/TXT | Não habilitar sem canal/finalidade autorizados; a página consultada restringe automação | Não confirmado | Não confirmado | Não confirmado | Não confirmado | `needs-info` |
| COTAHIST mensal | Arquivo histórico B3 | Não confirmado | Não confirmado | Não confirmado | Não confirmado | Não confirmado | `needs-info` |
| COTAHIST diário | Arquivo histórico B3 | Não confirmado | Não confirmado | Não confirmado | Não confirmado | Não confirmado | `needs-info` |
| BVBG.028.02 / InstrumentsConsolidated | Publicação oficial B3 XML ISO 20022 | Não confirmado | Não confirmado | Não confirmado | Não confirmado | Não confirmado | `needs-info` |
| Tradable Security List | Publicação oficial B3 CSV/JSON | Não confirmado | Não confirmado | Não confirmado | Não confirmado | Não confirmado | `needs-info` |
| BVBG.086.01, BVBG.186.01, BVBG.087.01 e composições | Publicações oficiais específicas | Fora do conjunto operacional inicial | Não confirmado | Não confirmado | Não confirmado | Não confirmado | Fora do escopo 0.0.1 |

`needs-info` não significa que todos os canais têm a mesma restrição ou que todos são necessariamente indisponíveis. Significa que a evidência ainda não autoriza habilitar aquela combinação de canal e finalidade. O catálogo precisa conservar estado independente para `acquisition`, `raw_retention`, `processing`, `rest`, `mcp` e `third_party_delivery`.

## Política operacional

- Canal não habilitado não pode ser adquirido, processado ou usado para servir consultas. A configuração administrativa não substitui evidência.
- Testes determinísticos usam fixtures claramente sintéticas ou cujo uso esteja autorizado; não fazem download ao vivo.
- Importação local continua sendo ferramenta apenas administrativa e exige artefato oficial obtido de forma permitida, preservando hash, origem declarada, manifesto, publicação e proveniência. Não há endpoint de upload a consumidores.
- Uma aprovação explícita do mantenedor pode autorizar um corpus local específico para desenvolvimento interno, aquisição manual, retenção raw e processamento. Essa autorização deve ser registrada no canal e limitada ao corpus identificado; não comprova direitos gerais nem habilita automação, REST, MCP ou entrega a terceiros. A publicação interna de uma revisão registra a visão selecionada do sistema e é separada da elegibilidade de cada transporte.
- O console administrativo pode ler uma revisão publicada por rotas internas específicas quando todas as fontes da revisão mantêm elegibilidade para `processing` e `raw_retention`. Essas rotas exigem credencial admin e não alteram as finalidades REST, MCP ou `third_party_delivery`.
- A rota administrativa `POST /api/v1/b3/discovery/candidates/register-local` registra essa attestation para um arquivo selecionado da pasta local, com escopo limitado ao canal criado. Ela não muda o estado `needs-info` dos canais externos nem preenche as finalidades de entrega.
- Um endpoint pode existir no software mesmo quando não há canal elegível. O catálogo/capabilities deve declarar a família indisponível e as consultas não devem fabricar resultados.
- Ativar uma finalidade não ativa automaticamente as demais. Permissão de download manual não implica automação; automação não implica retenção histórica; retenção não implica distribuição REST/MCP ou a terceiros.
- O software 0.0.1 pode ser concluído antes da resolução dessas evidências. Disponibilizar dados reais externamente exige resolver a linha correspondente da matriz antes de habilitá-la.

## Evidência necessária para ativar um canal

Registrar canal exato, domínio/fluxo autorizado, família e revisão de layout, finalidade, vigência, gratuidade, limites de automação, retenção/backups, atribuição, acesso REST/MCP, redistribuição a terceiros, documento/URL e data da análise. Uma conclusão negativa deve manter a finalidade em `restricted` ou `ineligible` e descrever se há um caminho alternativo permitido.

## Comentários

- 06/11/2026 — A matriz foi separada do critério de conclusão do software. A pendência externa bloqueia somente ativação/entrega do canal correspondente; fixtures sintéticas podem validar toda a arquitetura 0.0.1.
