# Pesquisa de fontes oficiais para o Tucano B3 histórico

Status: needs-triage

Data da pesquisa: **05/11/2026**, America/Sao_Paulo.

Este relatório fundamenta a [proposta de PRD](PRD.md). Distingue fatos documentados, observações do repositório e recomendações. Não constitui confirmação de cobertura integral, autorização de coleta ou homologação dos conectores.

## 1. Método e limites

- Revisão de `PRODUCT.md`, `docs/b3_prd.md`, README, módulos de aquisição, parsers, resolução, worker, REST, MCP, migrations, testes e Compose do `tucano-b3`, na base local cujo último commit era `cca1459`. Havia alterações não commitadas e outras ocorreram durante a sessão; a análise estática não equivale a auditar um commit imutável de toda a árvore.
- Revisão do produto, contextos de ingestão e atualizações e ADRs 0003, 0004, 0007 e 0008 do `tucano-cvm`.
- Pesquisa na web e leitura de páginas, glossários, catálogos e políticas da própria B3. O documento de política regulatória hospedado em `api.mziq.com` é uma publicação da B3; o ofício R08 indica a seção de governança do RI como seu ponto de distribuição.
- Download de quatro PDFs de documentação para extração de texto e conferência de conteúdo. Nenhum arquivo de negociações foi baixado, nenhum intervalo histórico foi varrido e nenhum endpoint de dados foi certificado nesta sessão.
- Resultados de busca podem conter cache. Páginas que anunciam uma série ou oferecem um seletor de datas não comprovam a existência de todos os arquivos desse intervalo.
- Catálogos antigos são evidência do leiaute daquela versão. A versão mais recente aplicável a cada arquivo ainda precisa ser homologada.

Não foram usados mirrors, Yahoo Finance, APIs de terceiros ou dados de corretoras como evidência de preços ou como proposta de aquisição.

## 2. Conclusões que mudam o desenho

| Achado | Consequência para o produto |
| --- | --- |
| A B3 apresenta histórico de cotações desde 1986, nominal e sem ajustes retroativos [R01](#r01) | COTAHIST é a melhor base candidata para o histórico longo. Isso não comprova cobertura integral por ativo |
| A distribuição anual, mensal e diária possui horizontes diferentes [R02](#r02) | Backfill anual; granularidades menores para manutenção e reconciliação, quando disponíveis |
| Há restrições explícitas à automação e à difusão de dados [R01](#r01), [R03](#r03), [R04](#r04) | Viabilidade de aquisição e redistribuição precisa ser resolvida por fonte e finalidade |
| O Security Definition Report documenta o Tradable Security List em CSV/JSON [R09](#r09) | A lista é uma fonte importante de identidade; não é apenas uma marca de negociabilidade |
| O cadastro consolidado possui glossário próprio [R10](#r10) | Não tratar `InstrumentsConsolidated` e `BVBG.028` como aliases de um mesmo contrato |
| O catálogo de instrumentos inclui identificação de distribuição [R11](#r11) | ISIN isolado é insuficiente para certas relações; preservar distribuição e contexto de mercado |
| O trailer do COTAHIST inclui os registros de controle [R12](#r12) | Corrigir a tolerância genérica a duas convenções no PRD anterior |
| Existem várias publicações diárias de boletins [R14](#r14) | Uma data pode ter edições distintas; a hora de download não identifica a edição oficial |
| A política regulatória de 2025 prevê prazos por grupo de dados [R07](#r07) | A notícia de 2019 que menciona dez dias não pode definir a retenção atual |
| A página de índices tem diferenças entre idiomas [R16](#r16) | Não prometer uma data inicial única sem verificar os arquivos |
| O cadastro usa `MktCptlstn` com descrição de capital social [R10](#r10) | Não produzir “valor de mercado” por tradução literal do identificador em inglês |

## 3. Fontes e evidências

<a id="r01"></a>
### R01 — Cotações históricas e aviso de automação

Fonte: [Cotações históricas — B3](https://b3.com.br/main.jsp?lumA=1&lumII=2C9E0371634B781701634B7C27111B5F&lumPageId=8A6A8C244DE7C289014DE80C873C130E).

**Documentado:** histórico desde 1986; preços na moeda e forma de cotação da época, sem ajustes por inflação ou proventos. A página também apresenta proibição expressa de coleta automatizada, abrangendo finalidades comerciais e não comerciais.

**Implicação proposta:** selecionar COTAHIST pelo valor histórico, mas deixar a automação condicionada à confirmação de uma forma de aquisição permitida. A afirmação “desde 1986” descreve a série oferecida; não prova continuidade por instrumento nem disponibilidade efetiva de cada ZIP.

<a id="r02"></a>
### R02 — Janelas de distribuição do COTAHIST

Fontes: [Séries Históricas — B3](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/market-data/historico/mercado-a-vista/series-historicas/) e [formulário oficial de arquivos](https://bvmf.bmfbovespa.com.br/pt-br/cotacoes-historicas/FormSeriesHistoricasArq.asp).

**Documentado:** o formulário oferece séries anuais; o ano corrente é acumulado até o último dia útil disponível. Mensais são apresentados como últimos doze meses e diários como ano corrente.

**Limite:** não houve download de ZIPs para confirmar esses horizontes na prática. Não supor que padrões de URL proporcionem diários de qualquer ano. Os nomes `COTAHIST_A{AAAA}.ZIP`, `COTAHIST_M{MMAAAA}.ZIP` e `COTAHIST_D{DDMMAAAA}.ZIP` permanecem candidatos de integração a homologar.

<a id="r03"></a>
### R03 — Hub de dados públicos

Fonte: [Hub de dados públicos — B3](https://www.b3.com.br/main.jsp?doui_processActionId=setLocaleProcessAction&locale=pt_BR&lumA=1&lumII=8AE490CA9A5E21B5019AEBE12A9C42A6&lumPageId=8AE490CA9A5E21B5019AEBE12A684286).

**Observado:** o portal reúne boletins, cadastros, índices e outras consultas. Também exibe restrição expressa à coleta automatizada.

**Implicação proposta:** o Hub serve como catálogo editorial de descoberta de fontes. Sua existência não equivale a licença aberta de dados ou a contrato de API.

<a id="r04"></a>
### R04 — Termos de uso institucionais

Fonte: [Termos de Uso — B3](https://www.b3.com.br/pt_br/termos-de-uso-e-protecao-de-dados/termos-de-uso/).

**Documentado:** os termos reservam direitos sobre o conteúdo, delimitam uso pessoal e exigem autorização para usos comerciais; a seção de difusão de dados restringe distribuição, republicação e fornecimento de bases e produtos a terceiros sem consentimento expresso.

**Inferência para esta proposta:** não existe evidência suficiente nesta pesquisa para declarar permitida uma API externa, mesmo gratuita. Isso também não permite concluir que todo uso histórico tenha a mesma licença. É necessário verificar o instrumento aplicável a cada fonte, canal e uso.

Não se tomou o termo do site de RI, nem o de outros produtos do grupo B3, como substituto deste termo institucional.

<a id="r05"></a>
### R05 — Políticas de Market Data vigentes na data da pesquisa

Fontes: [página de políticas e contratos](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/market-data/distribuidores/politica-comercial-e-contratos/) e [Política de Consumo, arquivo V1.2.1](https://www.b3.com.br/data/files/4F/25/23/55/5D881A10F75B881AAC094EA8/Politica%20de%20Consumo%20de%20Market%20Data%20B3%20V1.2.1.pdf).

**Conferido:** a página separa políticas anteriores e posteriores a 01/11/2026. O link de consumo vigente aponta para arquivo com `V1.2.1` no nome; a tabela interna identifica versão 1.2 com início em 01/11/2026. O PDF foi obtido diretamente e lido localmente porque o leitor web excedeu seu limite de tamanho. As seções 3 e 13 tratam também de dados históricos, fim de dia e desenvolvimento de produtos.

**Limite:** não extrapolar automaticamente a incidência desse contrato de feed de Market Data a todo arquivo regulatório público. A versão V1.1 encontrada por busca não foi usada como versão vigente.

<a id="r06"></a>
### R06 — UP2DATA contratado versus dados públicos

Fontes: [Contratação e Preços](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/up2data/perguntas-frequentes/contratacao-e-precos.htm) e [Dados disponíveis](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/up2data/dados-disponiveis/).

**Documentado:** UP2DATA possui contratação, canais e condições de uso e distribuição. A página de dados disponíveis distingue o canal regulatório público dos demais canais e identifica arquivos legados, inclusive a família BVBG, com materiais de migração.

**Implicação proposta:** não chamar o portal `arquivos.b3.com.br` inteiro de “UP2DATA gratuito”. Um arquivo regulatório publicado no site e a entrega de dados pelo serviço contratado são modalidades diferentes. Os materiais de migração de legado justificam avaliar adaptadores por família, sem impor migração automática para um canal pago.

As planilhas de mapeamento de legado foram localizadas, mas seu conteúdo não foi inspecionado nesta sessão.

<a id="r07"></a>
### R07 — Política de divulgação de dados regulatórios do Listado

Fonte: [Política da B3, revisão de 12/06/2025](https://api.mziq.com/mzfilemanager/v2/d/5fd7b7d8-54a1-472d-8426-eb896ad8a3c4/651805d7-853f-bda1-28a0-28e4eb16bd4a?origin=1), especialmente seções 5.2 e 6.

**Documentado:** a política prevê acesso público e gratuito; dados detalhados com atraso de até quinze minutos e retenção de vinte dias úteis; agregados diários disponíveis até o início da negociação seguinte e mantidos por dezoito meses. Os grupos possuem definições específicas.

**Limites:** isso não é uma garantia de dezoito meses para cada endpoint, cadastro, índice ou BVBG. Tampouco é autorização automática para redistribuição ou licença irrestrita de derivados. A viabilidade técnica de cada canal precisa de verificação própria. O documento contém uma inconsistência editorial na data da primeira versão; a revisão e vigência de junho de 2025 estão identificadas separadamente.

<a id="r08"></a>
### R08 — Origem da política e cuidado com documentação antiga

Fontes: [Ofício Circular 202/2023-PRE](https://www.b3.com.br/data/files/77/C6/6A/38/5012C810C96B50C8DC0D8AA8/OC%20202-2023%20PRE%20Publica%C3%A7%C3%A3o%20das%20Pol%C3%ADticas%20de%20Divulga%C3%A7%C3%A3o%20de%20Dados%20Regulat%C3%B3rios%20da%20B3%20%28PT%29.pdf) e [Disponibilização de dados para download, de 14/11/2019](https://www.b3.com.br/pt_br/noticias/dados-para-download.htm).

**Documentado:** o ofício apresenta as políticas de divulgação e remete à governança do RI. A notícia antiga anunciava dados gratuitos com histórico de dez dias.

**Implicação proposta:** usar a notícia de 2019 como contexto de lançamento, nunca como fonte única de um contrato atual de retenção. Pesquisa regulatória não transforma dados CVM em fonte de ingestão deste produto.

<a id="r09"></a>
### R09 — Tradable Security List / Security Definition Report

Fonte: [Security Definition Report — JSON and CSV, v1.0.1, 14/10/2024](https://www.b3.com.br/data/files/09/37/C5/F8/CDAB29106EEC8429AC094EA8/Security%20Definition%20Report-v1.0.1-enUS.pdf), seções 4–6.

**Documentado:** publicação por sessão, antes das 8h; acesso pela Pesquisa por Pregão sob o nome Tradable Security List; CSV e JSON. `Symbol` e `SecurityID` são obrigatórios; os demais campos são opcionais. O dicionário inclui qualificadores de identificação, ISIN, subjacentes, pernas, atributos de preço e grupos repetidos.

**Implicação proposta:** avaliar CSV/JSON como representação importante do cadastro de negociação. Tratar grupos repetidos e sentinelas conforme essa versão. O código de evento corporativo não substitui um histórico de eventos com valores e direitos.

**Limite:** o documento não comprova profundidade histórica ilimitada nem equivalência automática com todos os atributos do BVBG.028. Dizer que os preços foram lidos sem `f64` continua sendo requisito interno, independentemente de o dicionário usar tipos como `Double`.

<a id="r10"></a>
### R10 — InstrumentsConsolidatedFile público

Fonte: [Glossário Cadastro de Instrumentos — Listados, 02/02/2024](https://www.b3.com.br/data/files/52/74/1E/14/4BA6D8103152D4C8AC094EA8/Glossario%20InstrumentsConsolidatedFile%202024.pdf), pp. 3–9.

**Documentado:** CSV de cadastro divulgado antes da abertura e após o encerramento; possui ISIN, distribuição, fator de preço, prazo de liquidação e vários atributos. O glossário consultado não contém uma coluna `SecurityID`. Descreve `MktCptlstn` como capital social. O histórico de revisões distingue lote de alocação de outras noções de lote.

**Implicação proposta:** fonte complementar, com identidade temporal própria e cruzamento explícito; não fabricar `security_id` a partir de ticker. Preservar a semântica oficial dos campos e não expor `MktCptlstn` como valor de mercado calculado.

<a id="r11"></a>
### R11 — BVBG.028.02 InstrumentReport

Fonte: [Catálogo de Cadastro de Instrumento, v2.6, 24/10/2017](https://www.b3.com.br/data/files/0B/A1/CA/73/86072710547B5127AC094EA8/Catalogo-de-Mensagens-Cadastro-de-Instrumento-Versao-2.6.pdf), especialmente identificação financeira e atributos de ações.

**Documentado:** estruturas por categoria de instrumento, identificação financeira e local de listagem. `DistributionIdentification` participa da identificação de instrumentos com depositária em conjunto com ISIN.

**Implicação proposta:** manter o registro B3 como raiz de identidade, preservar distribuição e qualificadores, e documentar o mapeamento para Tradable Security List e cadastro consolidado. A versão do catálogo é de 2017: não foi afirmado que seja a mais recente.

<a id="r12"></a>
### R12 — Leiaute COTAHIST

Fonte: [Leiaute Cotações Históricas, revisão de 05/10/2020](https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf), pp. 3–6.

**Documentado:** registros de 245 bytes, tipos 00, 01 e 99; `PREULT` corresponde ao último negócio; `PRAZOT` preserva prazo de termo; `MODREF` registra a moeda; `FATCOT` expressa o fator de cotação; `PTOEXE` tem seis casas implícitas; `DISMES` registra distribuição. `CODISI` pode representar código interno e passa a representar ISIN a partir de 15/05/1995. O total do trailer inclui header e trailer.

**Implicação proposta:** parser estrito por versão; distinguir código histórico de ISIN validado; não converter fator de cotação em ajuste corporativo. A composição exata da chave natural é uma decisão do produto a testar no corpus, não uma chave de banco oficialmente garantida pelo PDF.

<a id="r13"></a>
### R13 — PriceReport, Simplified Price Report e IndexReport

Fonte: [Catálogo Preços e Índices, v1.2.1 no arquivo, 07/10/2021](https://www.b3.com.br/data/files/81/91/9A/17/27B5C710BD0885C7AC094EA8/Catalogo_precos_v1.2.1.pdf).

**Documentado:** arquivos `BVBG.086`, `BVBG.186` e `BVBG.187` usam a família de mensagem `bvmf.217.01`; `BVBG.087` usa `bvmf.218.01`. O catálogo diferencia relatórios completos, resumidos e índices. O relatório de preços inclui prazo de liquidação.

**Implicação proposta:** versões e famílias permanecem explícitas. Não inferir equivalência de todas as colunas ou sessões porque dois arquivos compartilham um schema XML. A homologação deverá selecionar XSDs e catálogos aplicáveis ao período.

<a id="r14"></a>
### R14 — Pesquisa por Pregão e edições

Fontes: [Pesquisa por Pregão](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/market-data/historico/boletins-diarios/pesquisa-por-pregao/pesquisa-por-pregao/) e [Ofício Circular 055/2026-VNC, 29/10/2026](https://www.b3.com.br/data/files/BE/16/C1/59/2F881A10F75B881AAC094EA8/OC%20055-2026-VNC%20NOVOS%20HORARIOS%20DE%20NEGOCIACAO_PT.pdf).

**Documentado:** o portal lista os boletins e a lista negociável, com seleção de datas retroativas. O ofício apresenta múltiplos horários aproximados para BVBG.086, além dos demais boletins, e exceções operacionais.

**Implicação proposta:** modelar edições e completude por sessão; nomes de ZIP e horário de coleta não bastam para estabelecer precedência. A profundidade retroativa de cada família não foi medida. O aviso de automação também aparece no portal.

<a id="r15"></a>
### R15 — Calendário B3

Fontes: [Calendário de negociação 2026](https://www.b3.com.br/main.jsp?doui_processActionId=setLocaleProcessAction&locale=pt_BR&lumA=1&lumII=8A80CB81633FBF0B0163402956733E3A&lumPageId=8A6882694E91F2D4014E9248DBB001C9) e [errata 003/2026-VNC](https://www.b3.com.br/data/files/FC/55/3B/12/7FE9B9109B5E99B9AC094EA8/OC%20003-2026-VNC%20ERRATA_CALENDARIO%20DE%20FERIADOS%20EM%202026%20E%20FUNCIONAMENTO%20DA%20B3%20EM%2018022026%20QUARTAFEIRA%20DE%20CINZAS_PT.pdf).

**Documentado:** calendário distingue atividades e segmentos, possui horários especiais e pode receber erratas.

**Implicação proposta:** calendário versionado por mercado e atividade. Feriados civis ou finais de semana isoladamente não bastam. A falta de calendário histórico verificado precisa permanecer como desconhecida, sem fabricar um denominador de cobertura.

<a id="r16"></a>
### R16 — Estatísticas históricas de índices

Fontes: [Evolução diária em português](https://sistemaswebb3-listados.b3.com.br/indexStatisticsPage/daily-evolution/IBOV?language=pt-br), [página em inglês](https://sistemaswebb3-listados.b3.com.br/indexStatisticsPage/daily-evolution/IBOVESPA?language=en-us) e [variação anual](https://sistemaswebb3-listados.b3.com.br/indexStatisticsPage/variation/IBOVESPA?language=pt-br).

**Observado:** a descrição em português menciona janeiro de 1988; a descrição em inglês menciona janeiro de 1998. A página anual explica adequações históricas de escala para divulgação.

**Implicação proposta:** pesquisar os arquivos e identificar a definição da série antes de declarar início. Essa fonte pode complementar um IndexReport recente, mas seria uma expansão explícita do catálogo permitido atual. Não equiparar ponto de índice, cotação de ETF e cotação de ação.

<a id="r17"></a>
### R17 — Composições definitivas e prévias

Fonte: [Nova carteira do Ibovespa, publicação de 08/09/2026](https://www.b3.com.br/pt_br/noticias/ibovespa-8AA8D0CDA023AA8301A08263E30959C0.htm).

**Documentado:** a publicação distingue carteira definitiva, período de vigência, data-base do fechamento e prévias; oferece arquivos de composição.

**Implicação proposta:** distinguir data de publicação, referência de pesos, vigência e estágio da carteira. Uma prévia não pode substituir composição efetiva. A composição atual não resolve automaticamente uma consulta histórica. A periodicidade e regras devem ser versionadas, em vez de cristalizadas a partir de uma única notícia.

<a id="r18"></a>
### R18 — Eventos corporativos

Fonte: [Dividendos e Outros Eventos Corporativos — portal oficial](https://sistemaswebb3-listados.b3.com.br/dividensOtherCorpActPage/).

**Documentado:** página destaca emissores atualizados nos últimos sete dias e orienta consulta por emissor para períodos maiores.

**Implicação proposta:** não confundir essa janela editorial com um conjunto histórico completo de eventos. A existência da consulta não comprova uma fonte gratuita, completa, automatizável e redistribuível de fatores de ajuste. Eventos e preços ajustados continuam fora da entrega proposta inicial.

<a id="r19"></a>
### R19 — TradeInformationConsolidated

Fonte: [Glossário de Negócios Consolidados, revisão de 15/05/2023](https://www.b3.com.br/data/files/F8/42/7A/61/FEC4A8103234E0A8AC094EA8/TradeInformationConsolidatedFileV3_PT.pdf).

**Documentado:** ZIP/CSV com estatísticas da sessão regular e divulgação noturna. O glossário diferencia quantidade de negócios (`TradQty`), quantidade de títulos/contratos (`FinInstrmQty`) e volume financeiro. Não lista preço de abertura. Cotação de ajuste de futuro tem significado próprio.

**Implicação proposta:** candidato a expansão futura para estatísticas recentes, sem substituir COTAHIST ou fundir sessões. Não interpretar `AdjustedQuote` como preço histórico de ação ajustado por dividendos. Sua inclusão exige alteração explícita do catálogo de fontes aceitas.

<a id="r20"></a>
### R20 — Glossários e contratos de distribuição

Fontes: [Índice oficial de glossários](https://www.b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/market-data/consultas/boletim-diario/dados-publicos-de-produtos-listados-e-de-balcao/) e [Perguntas frequentes UP2DATA](https://b3.com.br/pt_br/market-data-e-indices/servicos-de-dados/up2data/perguntas-frequentes/).

**Documentado:** os glossários são específicos por família. A FAQ distingue serviço contratado e bases históricas não expostas no site, para as quais menciona distribuidores.

**Implicação proposta:** usar os glossários públicos para a semântica, sem assumir que qualquer item do catálogo tenha histórico completo gratuito. Uma compra histórica a distribuidor contrariaria a restrição econômica e de aquisição primária deste projeto.

## 4. Registro de verificação dos PDFs

Os hashes abaixo identificam os bytes de documentação lidos nesta pesquisa. Não são checksums publicados pela B3, assinaturas de autenticidade ou evidência de homologação de dados. PDFs não foram copiados para o repositório.

| Documento | Páginas | SHA-256 calculado localmente |
| --- | --- | --- |
| Política Consumo, nome V1.2.1 | 57 | `f9d32cf68575e4713ba2511be49753c7ab39edd1edd389c2dab53f802015a5da` |
| Leiaute COTAHIST | 10 | `8b9633c44e1dc2229fb529931b4deb1d29fda9305a027d9c2c90a7ca96e8268f` |
| Catálogo instrumentos v2.6 | 101 | `271fbae40ccf0387c9455e860aacc1ea2a9e4ab12af70519ce4b4e77b61c99c3` |
| Catálogo preços v1.2.1 | 30 | `8425205c6b2d1ec20d92128255f66089c9d9ad45d26e2fe897a56fae05ff3807` |

## 5. Validações ainda necessárias

1. Confirmar com a B3, por fonte e finalidade, condições para coleta automatizada, persistência, uso analítico, REST/MCP, distribuição a terceiros e eventuais custos. Um parecer ou resposta deve delimitar o canal e o uso; “é público” não basta.
2. Após resolver a forma de acesso, verificar um corpus pequeno: ano antigo, transição monetária, ano recente fechado, ano em curso e sobreposição diário/mensal/anual.
3. Homologar catálogos/XSDs por período; testar namespaces e variantes reais, não apenas nomes de tags.
4. Medir profundidade por família, formato, edição e data; registrar ausências e falhas separadamente.
5. Verificar o mapeamento de `SecurityID`, fonte do identificador, bolsa, ISIN e distribuição entre os cadastros, com colisões e instrumentos expirados.
6. Confirmar os fluxos de download do canal aprovado. A sequência `requestname`/token e endpoints do frontend mencionados no PRD anterior não foram certificados como APIs públicas estáveis.
7. Verificar retenção de raw, compartilhamento de fixtures e distribuição de exports. A licença do software não concede licença dos dados.
8. Não anunciar preço ajustado, histórico completo de eventos, reconstrução point-in-time anterior à captura ou cobertura integral com base apenas nesta pesquisa.

## Comments

- 05/11/2026 — Solicitação: repensar o PRD com pesquisa profunda, dados oficiais gratuitos e históricos, preservando a stack e usando `tucano-cvm` como referência de resultado e organização.
- 05/11/2026 — Resultado: fontes e limitações registradas; conectores, permissões e cobertura histórica continuam pendentes de homologação. A pesquisa não implementou alterações de runtime.
