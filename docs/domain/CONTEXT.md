# Vocabulário de histórico de mercado B3

Glossário de apoio ao [PRD vigente](../b3_prd.md), adotado em 06/11/2026. Define o vocabulário de domínio utilizado pelos requisitos; não declara todas as capacidades implementadas.

Para guias narrativos de consumo e operação, veja o [site Docusaurus](../../docusaurus/docs/intro.md). Este glossário e o PRD continuam sendo as fontes de vocabulário e requisitos do projeto.

## Language

### Instrumentos e negociação

**Instrumento B3**:
Unidade financeira identificada no registro oficial da B3, contextualizada pelos qualificadores de identificação e pela sua história.
_Evitar_: Empresa, ticker ou ação como sinônimos universais.

**Identificador de instrumento**:
Código que identifica ou ajuda a identificar um instrumento em um contexto e período, como security_id, ISIN ou ticker.
_Evitar_: Chave eterna, identidade sem contexto.

**Ticker**:
Código de negociação publicado para determinado contexto de mercado e período; pode mudar ou ser reutilizado.
_Evitar_: Identidade permanente do ativo.

**Distribuição do papel**:
Identificação do estado de direitos de um papel conforme a fonte B3, distinta de uma revisão editorial de arquivo.
_Evitar_: Versão do download, evento de dividendo.

**Fotografia de cadastro**:
Representação dos instrumentos e atributos divulgados por uma fonte em uma referência ou edição específica.
_Evitar_: Cadastro de todos os tempos.

**Janela de validade de identificador**:
Período no qual a associação entre identificador e instrumento é sustentada por evidência admissível.
_Evitar_: Intervalo de observação como prova automática de vigência.

**Registro de negociação histórica**:
Fato de preço, quantidade ou estatística referente a uma data e granularidade de negociação publicada pela B3.
_Evitar_: Linha de ação sem mercado, candle genérico.

**Preço nominal bruto**:
Preço histórico na moeda e base de cotação da época, sem ajuste retroativo por inflação ou eventos corporativos.
_Evitar_: Preço ajustado, retorno total.

**Fator de cotação**:
Quantidade de referência à qual um preço publicado se refere, conforme a semântica da fonte.
_Evitar_: Fator de ajuste corporativo.

**Último preço**:
Preço do último negócio conforme o campo publicado, sem equivalência automática com qualquer outra medida de fechamento.
_Evitar_: Fechamento oficial como rótulo indiferenciado.

### Fontes e versões

**Família de fonte**:
Conjunto de publicações B3 com finalidade e contrato de interpretação próprios.
_Evitar_: Fonte genérica “preços” que apaga a distinção entre 086, 186 e COTAHIST.

**Publicação da fonte**:
Informação disponibilizada pela B3 para um escopo, possivelmente com edição e revisão identificáveis.
_Evitar_: Execução de coleta.

**Edição**:
Publicação distinguível dentro de uma referência, por exemplo uma rodada de boletim no mesmo pregão.
_Evitar_: Hora local de download.

**Artefato bruto**:
Conteúdo oficial recebido e preservado na forma em que foi adquirido.
_Evitar_: Resultado normalizado, evidência reconstruída.

**Member**:
Entrada individual de um arquivo distribuído, inclusive quando inserida em arquivos aninhados.
_Evitar_: Nome de arquivo sem o caminho de origem quando houver ambiguidade.

**Observação de fonte**:
Evidência de que determinado conteúdo foi encontrado em uma aquisição ou processamento.
_Evitar_: Alteração de negócio.

**Revisão publicada da base**:
Conjunto consistente de fatos e evidências selecionados para consulta segundo uma política identificada.
_Evitar_: Último arquivo baixado.

**Conflito de seleção**:
Divergência entre conteúdos candidatos ao mesmo fato sem precedência suficiente para escolher um deles.
_Evitar_: Duplicata comum, correção automática.

**Proveniência**:
Evidência que permite relacionar um fato ao conteúdo oficial, sua localização e interpretação.
_Evitar_: Apenas o nome “B3” como origem suficiente.

### Operação e qualidade

**Atualização pendente**:
Publicação nova ou possível revisão que aguarda análise ou tratamento operacional.
_Evitar_: Dado já alterado na base.

**Inserido**:
Fato válido que não existia na representação de domínio considerada.
_Evitar_: Toda linha processada.

**Alterado**:
Fato cujo conteúdo de negócio mudou em relação à representação anterior.
_Evitar_: Reobservado, vínculo de identidade reconstruído.

**Inalterado**:
Fato processado com conteúdo canonicamente equivalente ao já conhecido.
_Evitar_: Não processado, ignorado.

**Quarentena**:
Preservação de registro inválido ou não promovível com razão e localização suficientes para tratamento posterior.
_Evitar_: Exclusão de erro, sucesso vazio.

**Replay**:
Nova interpretação de evidência já preservada, mantendo sua origem e o histórico das tentativas.
_Evitar_: Novo download obrigatório, correção manual silenciosa.

**Resolução tardia**:
Vinculação de um fato preservado a um instrumento após chegada de evidência suficiente.
_Evitar_: Correção de preço, enriquecimento externo.

**Cobertura**:
Conhecimento demonstrável sobre quais escopos, fatos e relações estão disponíveis, incompletos ou desconhecidos.
_Evitar_: Primeiro e último dia como prova de continuidade.

**Elegibilidade de fonte**:
Compatibilidade documentada entre uma fonte, forma de acesso, finalidade de uso e restrição econômica do produto.
_Evitar_: Licença aberta deduzida de página pública.

**Carteira efetiva**:
Composição de índice divulgada para vigorar em determinado período, distinta de uma prévia e de fotografias diárias de pesos.
_Evitar_: Carteira atual como representação de todo o histórico.
