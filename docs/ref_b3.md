# Referência de campos B3

Seleção dos campos mais úteis para interpretar as famílias oficiais B3 aceitas pelo Tucano B3. Não substitui o leiaute, catálogo, schema ou glossário da versão aplicável ao arquivo. Campos abaixo preservam os nomes publicados pela fonte; a lista não declara que todos estejam disponíveis em todas as datas ou formatos.

O [glossário do domínio](domain/CONTEXT.md) explica os conceitos do projeto. O [PRD](b3_prd.md) contém os requisitos de identidade, precisão, proveniência e cobertura. Para versões e limitações das fontes consultadas, veja a [pesquisa](RESEARCH.md).

## 1. COTAHIST — cotações históricas

O leiaute oficial descreve registros de 245 bytes: `00` (header), `01` (cotação diária por papel-mercado) e `99` (trailer). O arquivo é de largura fixa. Decimais são implícitos no leiaute e devem ser interpretados pelo campo, sem conversão por ponto flutuante. A tabela resume os campos mais relevantes do registro `01`.

| Campo B3 | Significado | Cuidados de uso |
|---|---|---|
| Data do pregão | Data a que a cotação se refere (`AAAAMMDD`) | Não confundir com a data de geração do arquivo. |
| `CODBDI` | Código BDI que classifica o papel na emissão do boletim | Manter o código original e consultar a tabela da versão do leiaute. |
| `CODNEG` | Código de negociação do papel | Ticker contextual e temporal; não é chave permanente isoladamente. |
| `TPMERC` | Tipo de mercado em que o papel está cadastrado | Necessário para evitar misturar universos distintos. |
| `NOMRES` | Nome resumido da empresa emissora | Texto descritivo, não identidade de emissor para relacionamento externo. |
| `ESPECI` | Especificação do papel | Preservar o código de origem; não inferir classe apenas pelo sufixo do ticker. |
| `PRAZOT` | Prazo em dias do mercado a termo | Discriminador de fatos a termo; não descartar quando nulo ou preenchido. |
| `MODREF` | Moeda de referência usada na data do pregão | Moedas históricas não devem ser convertidas sem metodologia externa aprovada. |
| `PREABE`, `PREMAX`, `PREMIN`, `PREMED` | Preços de abertura, máximo, mínimo e médio | Preservar a escala implícita e a unidade/fator da fonte. |
| `PREULT` | Preço do último negócio no pregão | Não renomear como fechamento oficial genérico. |
| `PREOFC`, `PREOFV` | Melhores ofertas de compra e venda | São ofertas; não são preços de negócio. |
| `TOTNEG` | Número de negócios realizados | Contagem de negócios, diferente de quantidade de títulos. |
| `QUATOT` | Quantidade total de títulos negociados | Interpretar com o produto e a unidade da fonte. |
| `VOLTOT` | Volume total negociado | Valor financeiro com escala implícita do leiaute. |
| `PREEXE`, `INDOPC`, `DATVEN`, `PTOEXE` | Preço/valor de exercício, indicador de correção, vencimento e pontos de exercício | Campos de opções e termo têm semânticas próprias; não tratar pontos como preço simples. |
| `FATCOT` | Fator de cotação | Indica a base da cotação, por exemplo unitária ou por lote; não é fator de ajuste corporativo. |
| `CODISI` | Código do papel no sistema; ISIN ou código interno conforme o período | O leiaute informa que o código passa a ser ISIN a partir de 15/05/1995; não validar retroativamente todo valor como ISIN. |
| `DISMES` | Número de distribuição, ligado ao estado de direito vigente do papel | Parte do discriminador natural em COTAHIST; não é versão do arquivo. |

No trailer `99`, o total informado inclui header e trailer. O header e o trailer também carregam nome/origem do arquivo e data de geração. Validar a largura em bytes e os registros de controle antes de publicar o conteúdo. [Leiaute oficial COTAHIST, revisão de 05/10/2020](https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf) · [R01, R02 e R12](RESEARCH.md#r01)

## 2. Tradable Security List / Security Definition Report

A especificação consultada descreve JSON e CSV, grupos repetidos e campos opcionais. `Symbol` e `SecurityID` são campos centrais do registro; ausência de campo opcional não deve ser transformada em valor presumido.

| Campo | Significado | Cuidados de uso |
|---|---|---|
| `Symbol` | Símbolo/ticker do instrumento | Usar junto aos qualificadores e à referência da publicação. |
| `SecurityID` | Identificador do instrumento na fonte | Preservar namespace e qualificador; não presumir que outro arquivo use o mesmo domínio de IDs. |
| `SecurityIDSource` | Fonte/tipo do identificador | Parte da identidade do código. |
| `SecurityExchange` | Mercado associado ao instrumento | Não inferir equivalência entre mercados pela coincidência do ticker. |
| `ISINNumber` | ISIN publicado | Pode estar ausente; complementar com distribuição/contexto quando aplicável. |
| `Product`, `CFICode`, `SecurityGroup`, `SecurityType`, `SecuritySubType` | Produto e classificação do instrumento | Códigos de classificação dependem do dicionário e versão da fonte. |
| `SecurityDesc`, `Asset`, `Currency`, `SettlCurrency` | Descrição, ativo associado e moedas | Valores descritivos ou unidades; não são chaves de identidade por si sós. |
| `MaturityMonthYear`, `MaturityDate`, `IssueDate`, `StartDate`, `EndDate` | Datas de emissão, referência e validade conforme o campo | Preservar sentinelas documentadas, como `9999-12-31`, como sentinela; não data efetiva. |
| `StrikePrice`, `StrikeCurrency`, `PutOrCall`, `ExerciseStyle`, `ContractMultiplier` | Atributos de opções/derivativos | Decimais, moeda e códigos precisam permanecer tipados e exatos. |
| `PriceDivisor`, `MinPriceIncrement`, `MinOrderQty`, `MaxOrderQty` | Divisor, incremento e quantidades de negociação | Não confundir divisor com fator de ajuste histórico. |
| `MarketSegmentID`, `GovernanceIndicator`, `SecurityValidityTimestamp` | Segmento e atributos de negociação/validade | Interpretar de acordo com catálogo e data da publicação. |
| `NoApplIDs`, `NoInstrAttrib`, `NoUnderlyings`, `NoLegs`, `NoLotTypeRules` | Grupos repetidos | Manter a cardinalidade e os itens; não achatar em uma única coluna textual sem contrato. |

Os grupos incluem campos filhos como `ApplID`, `InstrAttribType`, `InstrAttribValue`, `UnderlyingSymbol`, `UnderlyingSecurityID`, `LegSymbol`, `LegRatioQty`, `LegSide`, `LotType` e `MinLotSize`. A especificação 1.0.1 é de 14/10/2024 e não prova profundidade histórica ilimitada. [Security Definition Report v1.0.1](https://www.b3.com.br/data/files/09/37/C5/F8/CDAB29106EEC8429AC094EA8/Security%20Definition%20Report-v1.0.1-enUS.pdf) · [R09](RESEARCH.md#r09)

## 3. Cadastro `BVBG.028.02` / InstrumentReport

O catálogo consultado é a versão 2.6, datada de 24/10/2017. Ele descreve `InstrumentReport` (`bvmf.100.02`) em XML e categorias de instrumentos. Esses nomes servem como referência do catálogo; a compatibilidade do parser deve ser verificada com o schema oficial aplicável ao período.

| Campo / elemento do catálogo | Significado | Cuidados de uso |
|---|---|---|
| `FinancialInstrumentIdentification` (`FinInstrmId`) | Bloco de identificação do instrumento | Preservar os subcampos e a origem. |
| `OtherIdentification` (`OthrId`) → `Identification` (`Id`) | Código sequencial de instrumento no sistema Trade Structure (`Security ID`) | Associar ao tipo/namespace e ao mercado publicados. |
| `PlaceOfListing` (`PlcOfListg`) → `MarketIdentifierCode` (`MktIdrCd`) | Mercado em que o instrumento está listado | Qualificador de identidade e resolução. |
| `ISIN` | ISIN do instrumento quando fornecido | Para instrumentos com depositária, considerar junto do código de distribuição. |
| `DistributionIdentification` (`DstrbtnId`) | Código de distribuição/versão do papel | Aplicável a instrumentos como ações e ouro segundo o catálogo; não há distribuição de derivativos. |
| `CFICode` (`CFICd`) | Classificação do instrumento | Manter o valor e a versão do dicionário. |
| `MeasureUnit` (`MeasrUnit`) | Unidade de medida da informação do instrumento | Relevante à interpretação de quantidades e fatores. |
| `PriceFactor` (`PricFctr`) | Base usada para apresentar o preço | Não confundir com ajuste por evento corporativo. |
| `TradingStartDate`, `TradingEndDate` | Início e fim de negociação segundo a fonte | Não equiparar a primeira/última observação local à validade oficial. |

O bloco categórico pode conter atributos adicionais diferentes entre ações, derivativos e outros instrumentos. Evitar uma projeção única que descarte particularidades da categoria. [Catálogo de Cadastro de Instrumento v2.6](https://www.b3.com.br/data/files/0B/A1/CA/73/86072710547B5127AC094EA8/Catalogo-de-Mensagens-Cadastro-de-Instrumento-Versao-2.6.pdf) · [R11](RESEARCH.md#r11)

## 4. `InstrumentsConsolidatedFile`

O glossário de dados públicos consultado em 02/02/2024 descreve um CSV de cadastro de listados, divulgado antes da abertura e após o encerramento. Os termos abaixo são selecionados do glossário; a versão do glossário deve acompanhar qualquer interpretação de produção.

| Código no glossário | Termo | Cuidados de uso |
|---|---|---|
| `RptDt` | Data do relatório | Referência da fotografia, não início presumido de validade. |
| `TckrSymb` | Símbolo/ticker | Código conveniente de consulta, mas mutável e reutilizável. |
| `Asst`, `AsstDesc` | Ativo e descrição do ativo | Contexto útil especialmente para contratos; não é identificador externo. |
| `SgmtNm`, `MktNm`, `SctyCtgyNm` | Segmento, mercado e categoria | Preservar os três níveis de classificação. |
| `XprtnDt`, `XprtnCd` | Data e código de expiração | A semântica varia por contrato e formato do código. |
| `TradgStartDt`, `TradgEndDt` | Início e fim de negociação | Evidência cadastral da fonte, distinta de datas locais de observação. |
| `ISIN` | Identificador ISIN | Considerar distribuição e qualificadores quando aplicáveis. |
| `CFICd` | Código CFI | Classificação, não identidade única. |
| `DstrbtnId` | Identificação da distribuição | Preservar quando publicado; não aplicar a derivativos sem fundamento. |
| `PricFctr` | Fator de preço | Unidade de cotação; não é ajuste corporativo. |
| `MktCptlstn` | Capital social (`MarketCapitalisation` no nome inglês) | O glossário descreve capital social, não valor de mercado calculado. |

O glossário consultado não documenta `SecurityID` para esse arquivo. Não fabricar esse identificador a partir de ticker nem tratar a fotografia como substituto integral do `BVBG.028` ou da Tradable Security List. [Glossário InstrumentsConsolidatedFile, 02/02/2024](https://www.b3.com.br/data/files/52/74/1E/14/4BA6D8103152D4C8AC094EA8/Glossario%20InstrumentsConsolidatedFile%202024.pdf) · [R10](RESEARCH.md#r10)

## 5. `BVBG.086.01`, `BVBG.186.01` e `BVBG.087.01`

O catálogo de Preços e Índices consultado, de 07/10/2021, associa `BVBG.086.01`, `BVBG.186.01` e `BVBG.187.01` à mensagem `bvmf.217.01` e `BVBG.087.01` à `bvmf.218.01`. O compartilhamento de schema entre relatórios não os torna produtos equivalentes.

| Família | Campos e blocos exemplares | Cuidados de uso |
|---|---|---|
| `BVBG.086.01` PriceReport | Identificação (`SecurityID`, ticker e mercado), `DaysToSettlement`, `TradeQuantity`, volumes nacionais/internacionais, `OpenInterest`, quantidade de instrumentos, melhores ofertas e preços da sessão | `DaysToSettlement` distingue mercado a termo. Identificar edição e sessão; não renomear toda medida final como fechamento. |
| `BVBG.186.01` Simplified Price Report | Campos resumidos de preços e volume para ações conforme contrato próprio | Sem fallback para 086; a chave e a granularidade precisam ser homologadas separadamente. |
| `BVBG.087.01` IndexReport | Identificação do índice e blocos de valores publicados | Tipo de valor e unidade pertencem ao bloco; não converter referência de BDR/IOPV em fechamento de índice. |

O catálogo 1.2.1 descreve nomes e estruturas de mensagem. Antes de interpretar arquivos, confirmar XSD/catálogo vigente e campos existentes na família/edição. Não usar o catálogo de 2021 como afirmação de compatibilidade atual sem essa verificação. [Catálogo Preços e Índices v1.2.1](https://www.b3.com.br/data/files/81/91/9A/17/27B5C710BD0885C7AC094EA8/Catalogo_precos_v1.2.1.pdf) · [R13–R14](RESEARCH.md#r13)

## 6. Carteiras de índices

Uma composição deve manter, quando publicados ou comprovados: código do índice, data-base, publicação, estágio (prévia ou definitiva), janela de vigência, identificação do componente, peso e proveniência. Os nomes concretos e as unidades permanecem os da especificação do arquivo de cada canal.

Não propagar o peso de uma data por toda a vigência sem regra da fonte; não substituir uma carteira histórica pela atual; não considerar uma prévia como composição efetiva. A referência consultada demonstra as distinções de carteira definitiva, prévia, data-base e vigência, mas não prova uma série completa de composições. [Publicação B3 sobre carteira do Ibovespa](https://www.b3.com.br/pt_br/noticias/ibovespa-8AA8D0CDA023AA8301A08263E30959C0.htm) · [R17](RESEARCH.md#r17)

## Regras transversais de interpretação

- Usar `rust_decimal::Decimal` para preços, quantidades e fatores decimais; preservar escala e unidade sem arredondamento implícito.
- Manter nome/código original do campo junto à interpretação normalizada quando a linhagem exigir.
- Separar validade do identificador, referência da fotografia, pregão, publicação, edição, observação e ingestão.
- Respeitar as fronteiras entre `BVBG.086`, `BVBG.186`, `BVBG.087`, COTAHIST e as três famílias cadastrais.
- Não inferir dados CVM, CNPJ, empresa emissora ou eventos corporativos como âncoras de identidade deste produto.
- Para COTAHIST, manter preços nominais sem ajuste e a distinção entre `PREULT`, fator de cotação e valores de exercício.
- Dar proveniência ao fato até arquivo, member e linha/registro; manter `unresolved` e `ambiguous` quando faltarem evidências.

## Referências oficiais e notas

- [Pesquisa de fontes do Tucano B3](RESEARCH.md): resumo, data de consulta e limitações de R01–R20.
- [Leiaute COTAHIST — B3](https://www.b3.com.br/data/files/33/67/B9/50/D84057102C784E47AC094EA8/SeriesHistoricas_Layout.pdf), revisão 2, 05/10/2020.
- [Security Definition Report — B3](https://www.b3.com.br/data/files/09/37/C5/F8/CDAB29106EEC8429AC094EA8/Security%20Definition%20Report-v1.0.1-enUS.pdf), v1.0.1, 14/10/2024.
- [Catálogo Cadastro de Instrumento — B3](https://www.b3.com.br/data/files/0B/A1/CA/73/86072710547B5127AC094EA8/Catalogo-de-Mensagens-Cadastro-de-Instrumento-Versao-2.6.pdf), v2.6, 24/10/2017.
- [Glossário InstrumentsConsolidatedFile — B3](https://www.b3.com.br/data/files/52/74/1E/14/4BA6D8103152D4C8AC094EA8/Glossario%20InstrumentsConsolidatedFile%202024.pdf), 02/02/2024.
- [Catálogo Preços e Índices — B3](https://www.b3.com.br/data/files/81/91/9A/17/27B5C710BD0885C7AC094EA8/Catalogo_precos_v1.2.1.pdf), 07/10/2021.

Essas versões documentais são evidências para interpretação, não garantia de que sejam as últimas versões aplicáveis ao arquivo adquirido. O acesso público tampouco comprova automaticamente autorização de coleta, armazenamento ou redistribuição; ver [elegibilidade das fontes](release_source_eligibility.md).
