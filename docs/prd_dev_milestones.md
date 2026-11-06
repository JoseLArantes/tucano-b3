# Marcos de desenvolvimento do Tucano B3

Status: planejamento de execução, sujeito aos critérios de saída de cada marco.

Execução 0.0.1 iniciada no [plano de lançamento](0.0.1-plan.md). Decisão posterior do mantenedor: base nova e primeiro contrato público, sem migração dos registros/IDs anteriores. Os requisitos de preservação passam a reger o novo conjunto após seu lançamento; a reconstrução do legado não integra o caminho crítico desta entrega.

Data da revisão: **06/11/2026**, America/Sao_Paulo. Baseline histórica da transição: **`fdc804f`**. Implementação integrada em qualificação no commit **`7e93b0d`**; validação executada contra a árvore desse commit. Referência de produto: [PRD vigente, versão 2.0](b3_prd.md).

Este documento detalha os marcos G0–G7 da seção 20 do PRD a partir do código e da revisão existentes. Define sequência, dependências, entregas e evidências de conclusão; não é um PRD alternativo. O [plano 19](19-plan.md) orienta a avaliação e a migração do legado; as [issues 01–09](issues/README.md) continuam sendo o backlog rastreável. Requisitos de produto e compatibilidade prevalecem sobre propostas de implementação ainda não homologadas.

## 1. Baseline histórica da transição (`fdc804f`)

Esta seção preserva a fotografia que motivou o plano no commit `fdc804f`. Ela não descreve o estado atual depois da implementação de `0.0.1-plan.md`. Naquela base, o projeto estava na **transição entre a revisão da implementação existente e a construção do núcleo histórico publicável**.

| Área | Situação observada em `fdc804f` | Consequência para o desenvolvimento |
| --- | --- | --- |
| Stack e organização | Rust, serviços REST/CLI/worker/MCP, SQLx, PostgreSQL 16 e Compose existentes | Evoluir a implementação por capacidade; preservar módulos e contratos úteis |
| Revisão de aderência | Issue 09 registrada como concluída; avaliação e desenho de migração em `.scratch/prd-alignment/` | Usar os resultados como entrada; conclusão da revisão não conclui os marcos de produto |
| Testes e regressões | A avaliação registra 37 testes aprovados; há cobertura de precisão, hashes, constraints, auditoria, contratos e helpers de lock | Preservar essa cobertura e acrescentar cenários de pipeline; quantidade de testes não comprova os requisitos ausentes |
| Aquisição e parsing | Download ainda usa `resp.bytes().await?.to_vec()`; extração e parsers retêm coleções em memória | G2/G3 precisam demonstrar processamento limitado por bytes e registros |
| Arquivos aninhados | O limite de profundidade já produz erro visível, com regressão para member válido ao lado de aninhamento excessivo | Preservar a correção; não tratá-la como pendente apenas por constar em uma descrição anterior |
| Publicação e persistência | Existe apenas `0001_initial_b3_schema.sql`; não há migrations implementando o novo modelo de revisão | Propostas de `public_id`, ponteiro ativo e unicidade por revisão são desenho, não capacidade entregue |
| Atomicidade e exclusão | A transação de advisory lock é retida durante `run_single_date`; escritas internas usam outras conexões do pool | Preservar a exclusão existente e corrigir as fronteiras de confirmação; o lock não comprova rollback conjunto |
| Worker | Usa `Local::now()` e fins de semana; não consome o calendário persistido nem uma fila com lease/fencing | G5 começa junto de G2/G3, antes da primeira publicação pelo novo pipeline |
| Consultas | REST e MCP existem, com SQL duplicado entre transportes | Extrair serviços compartilhados e integrar a seleção de revisão sem regressão de contrato |
| Homologação de fontes | Existem fixtures sintéticas e uma fixture 086 baseada no catálogo; compatibilidade com corpus oficial representativo continua pendente | Separar teste unitário, aderência ao catálogo e homologação de arquivo oficial |
| MCP restrito | Inicialização dispensa migrations por inspeção; falta teste do processo com cliente e credencial somente leitura | Tratar a comprovação ponta a ponta como saída de G4 |
| Elegibilidade e cobertura | Permissões por finalidade e profundidade histórica não estão encerradas | Trabalho offline pode avançar; habilitação de fonte e lançamento dependem de G0 |

A informação sobre os 37 testes é **evidência registrada na avaliação histórica**, não uma execução realizada para elaborar este documento. A [matriz de aderência](../.scratch/prd-alignment/assessment.md) e o [desenho de migração](../.scratch/prd-alignment/migration.md) descrevem aquela etapa e não substituem a validação da implementação corrente.

## 2. Estado da implementação integrado em 06/11/2026

A transição de implementação foi superada em boa parte: o núcleo histórico tem migrations, modelos tipados, aquisição limitada, parsers incrementais, staging, revisões, jobs PostgreSQL, API, MCP e descoberta. Isso comprova capacidade de engenharia com fixtures e bancos descartáveis; **não** comprova elegibilidade de uso externo, completude do corpus ou homologação com publicação B3 real. O primeiro lançamento permanece bloqueado por G0 e pela qualificação final.

| Marco | Evidência de implementação | Pendência de aceite |
| --- | --- | --- |
| G0 — Viabilidade e corpus | Matriz por fonte/finalidade e validação fail-closed implementadas; [registro de elegibilidade](release_source_eligibility.md) documenta a análise | Confirmação externa para aquisição/retenção/distribuição a terceiros sem custo obrigatório; corpus e cutoff oficiais congelados |
| G1 — Modelo e contratos | Migrations `0004`–`0011`, fatos imutáveis/versionados, revisões e ponteiro ativo, relações de identidade reconstruíveis; testes cobrem correção, conflito, rejeição, rollback e rebuild sem alterar valores | Validar chaves naturais e cobertura de exceções no corpus oficial; completar critérios do PRD por fonte |
| G2 — Aquisição e replay | Hash incremental, raw endereçado por conteúdo, importação local confinada, inventário de ZIPs aninhados e limites de expansão/membros/metadados; HTTP determinístico testado | Homologar fontes, redirecionamentos e formatos reais; concluir medição de recursos no corpus de release |
| G3 — Núcleo histórico | COTAHIST streaming, registro de segurança tipado, staging em lotes, proveniência e publicação explícita; testes reais de processos cobrem lease expirada, worker obsoleto e publicação | Ingerir e reconciliar a amostra permitida 2025–2026, validar sobreposições e calendário, concluir aceite em corpus real |
| G4 — Produto de consulta | REST `/api/v1/b3/...` e MCP stdio usam query services compartilhados; auth separada, provenance, revision pinning, detail/identifiers/TSL e role MCP read-only cobertos | Smoke da imagem corrente e validação dos contratos com dataset publicado e evidências dos arquivos reais |
| G5 — Operação confiável | Jobs duráveis, lanes, retries, fencing, cancelamento, descoberta desacoplada, calendário desconhecido preservado; imagens API/worker/migrate buildadas; smoke API/worker; benchmark sintético concorrente; backup/restore seguido de consulta REST | Repetir medições com corpus permitido e infraestrutura-alvo antes de fixar budgets; calendário real segue pendente |
| G6 — Boletins de preços | Famílias existentes permanecem isoladas; não foram habilitadas no produto `0.0.1` | Fora do primeiro lançamento; elegibilidade, corpora e homologação próprios |
| G7 — Índices e carteiras | Parsing legado não equivale à integração na plataforma versionada do primeiro lançamento | Fora do primeiro lançamento; elegibilidade, corpora e homologação próprios |

Validação corrente: `docker compose run --rm cargo cargo fmt --all -- --check`, Clippy com `-D warnings`, e suíte completa em PostgreSQL 16.15 isolado passaram (**65 testes; sem falhas ou ignorados**). Imagens de release foram buildadas; health/auth, worker, publicação, carga de leitura concorrente e restore com 20.000 fatos sintéticos passaram. Os números e limites de sua aplicação constam no [plano do primeiro lançamento](0.0.1-plan.md) e no [guia operacional](release_operations.md). Homologação oficial e budget de produção continuam pendentes.

### Reutilização e limites herdados

Reutilizar somente capacidades que atendam ao PRD: precisão decimal, normalização, hashes determinísticos, auditoria granular, tratamento de chaves nulas, separação 086/186, aviso COTAHIST e identidade temporal. O namespace REST `/api/v1/b3` permanece conforme o PRD, mas os contratos, dados, IDs e estado de execução anteriores não são importados nem exigem compatibilidade com `0.0.1`. A partir do lançamento, as garantias de identidade e histórico passam a ser estáveis. O novo runtime usa serviços de domínio e coordenação PostgreSQL compartilhados.

O modelo de dados atual não será congelado para acomodar suas limitações. Chaves, associações reconstruíveis e regras de seleção precisam ser homologadas antes de novas constraints. `0001` permanece imutável; cada alteração estrutural exige nova migration e validação de upgrade com dados.

## Quadro histórico de marcos

Os estados abaixo descrevem a **prontidão na baseline histórica `fdc804f`** e não substituem a tabela de estado corrente acima nem os status de triagem das issues. “Parcial” naquela avaliação indicava código ou preparação aproveitável, com critérios de saída pendentes.

| Marco | Estado em `fdc804f` | Resultado de saída | Dependência efetiva | Issue |
| --- | --- | --- | --- | --- |
| G0 — Viabilidade e corpus | Pendente de evidência externa | Fonte, canal, finalidade e recorte elegíveis, sem custo de dados obrigatório | Independente; libera uso de corpus e disponibilização conforme a finalidade | [01](issues/01-source-viability.md) |
| G1 — Modelo e contratos | Parcial; decisões e corpus pendentes | Identidade, granularidade, revisões e migração especificadas e testáveis | Baseline da revisão; corpus de G0 para homologação definitiva | [02](issues/02-domain-and-contracts.md) |
| G2 — Aquisição e replay | Parcial; streaming e limites pendentes | Raw íntegro e imutável, inventário completo, aquisição e replay limitados | Contratos mínimos de G1; G0 para aquisição externa | [03](issues/03-artifact-acquisition.md) |
| G3 — Núcleo histórico | Parcial; publicação recuperável pendente | COTAHIST e cadastro disponível, com identidade, auditoria e revisão publicável | G1, G2, corpus permitido e coordenação mínima de G5.1 | [04](issues/04-historical-core.md) |
| G4 — Produto de consulta | Parcial; integração com novo núcleo pendente | REST/MCP consistentes, cobertura explicada e compatibilidade verificada | Revisão publicável de G3; trabalho de caracterização pode começar antes | [05](issues/05-query-product.md) |
| G5 — Operação confiável | Parcial; fila e calendário pendentes | Descoberta separada, recuperação, manutenção prioritária e operação verificável | G1; implementação conjunta com G2/G3, sem aguardar G4 | [06](issues/06-operations.md) |
| G6 — Boletins de preços | Código inicial; homologação posterior ao núcleo | 086/186 por edição, separados, com comparação explícita | Plataforma G1–G5 e G0 específico de cada fonte | [07](issues/07-price-reports.md) |
| G7 — Índices e carteiras | Código inicial; homologação posterior ao núcleo | 087 e composições com tipos, unidades, vigência e estágio | Plataforma G1–G5 e G0 específico de cada fonte | [08](issues/08-indices-and-portfolios.md) |

### Sequência e trabalho concomitante

```mermaid
flowchart TD
    E0[Consolidar evidencias da revisao] --> G1[G1 Modelo e contratos]
    G0[G0 Elegibilidade e corpus] -. Homologacao por fonte .-> G1
    G1 --> G2[G2 Aquisicao e replay]
    G1 --> G51[G5.1 Coordenacao minima]
    G2 --> G3[G3 Nucleo historico]
    G51 --> G3
    G0 -. Corpus permitido .-> G3
    G3 --> G4[G4 REST e MCP]
    G51 --> G52[G5.2 Descoberta e calendario]
    G2 --> G52
    G3 --> G53[G5.3 Operacao e recursos]
    G52 --> G53
    G4 --> L1[Primeiro lancamento G0 a G5]
    G53 --> L1
    G0 -. Finalidade do lancamento .-> L1
    L1 --> G6[G6 Boletins 086 e 186]
    L1 --> G7[G7 Indices e carteiras]
```

As setas indicam dependências de aceite, não impedem preparação independente. G0 corre em paralelo; fixtures sintéticas ou autorizadas permitem desenvolvimento offline de G1/G2/G5. G5.2 pode avançar com mocks e calendário de teste enquanto G3 é construído. G4 pode caracterizar os contratos atuais antes da integração final. G6 e G7 não dependem um do outro e não precisam esperar o backfill histórico inteiro, mas só entram em operação após comprovar a plataforma comum e a elegibilidade específica.

Não criar uma dependência circular entre “G3 completo” e “G5 completo”: G5.1 fornece coordenação e recuperação mínimas para G3; os ensaios integrados de manutenção, concorrência e restauração encerram G5.3 sobre o núcleo já publicável.

## 3. Próximos passos para concluir 0.0.1

A revisão da implementação e a integração do núcleo estão concluídas para qualificação no commit `7e93b0d`. O trabalho remanescente é fechar as evidências e os critérios de lançamento do [plano 0.0.1](0.0.1-plan.md), sem reabrir a migração do estado legado.

1. Resolver a elegibilidade gratuita por fonte, canal e finalidade. Enquanto permanecer `needs-info`, não adquirir nem expor dados reais por esse canal; manter o bloqueio externo.
2. Quando permitido, adquirir as publicações oficiais disponíveis, validar o calendário e congelar o manifesto com hashes, formatos, períodos, members e cutoff provado pelas próprias publicações.
3. Ingerir primeiro as fotografias de cadastro elegíveis e depois o corpus COTAHIST 2025 e 2026 até o cutoff demonstrado. Reconciliar natural keys, sobreposições e conflitos sem descartar fatos válidos não resolvidos.
4. Repetir a qualificação, os smoke tests, backup/restore e benchmark com o corpus permitido e o hardware-alvo; só então fixar budgets operacionais e aceitar W8/W9.
5. Atualizar issues 01–06 e o manifesto de release com evidências verificáveis. Habilitar acesso externo somente após aprovação de G0 e aceite de G1–G5.

**Condição de saída:** fonte e finalidade aprovadas, corpus/cutoff congelados, revisão validada e restaurável, contratos/documentação correspondentes ao que foi realmente qualificado. Sem evidência de G0, a implementação permanece pronta para homologação, mas a publicação externa continua bloqueada.

## 4. G0 — Comprovar viabilidade e formar corpus

Referência: PRD §§4–6 e 18; [issue 01](issues/01-source-viability.md).

**Resultado:** saber exatamente quais dados podem ser adquiridos, preservados e expostos, por qual canal e finalidade, respeitando a restrição de custo zero dos dados.

**Entrada:** matriz de fontes e pesquisa já existentes. Não depende de mudanças no código.

### Entregas

- **G0.1 — Catálogo de capacidade e elegibilidade:** registrar fonte, canal oficial, schemas, granularidades, finalidade, condições de aquisição/armazenamento/redistribuição, gratuidade, evidência e data de verificação. Usar `unreviewed`, `eligible`, `restricted` e `ineligible` conforme o PRD.
- **G0.2 — Manifesto de corpus:** relacionar arquivos/amostras, hashes, origem, condições de uso, períodos, edições, encodings, members e finalidade de cada fixture. Distinguir arquivo oficial, amostra baseada em catálogo e exemplo sintético.
- **G0.3 — Recorte liberado:** declarar fontes e períodos do piloto; registrar horizonte disponível, lacunas e condições de acesso. A cobertura anunciada desde 1986 não equivale a arquivos coletados e validados para todos os anos.

### Critérios de saída

- Cada fonte/finalidade habilitada tem evidência compatível com aquisição, retenção e consumo pretendidos. Download manual permitido não libera automaticamente automação ou API para terceiros.
- Corpus permitido permite homologar os formatos efetivamente escolhidos. Formato sem corpus continua pendente, sem impedir entregas offline independentes.
- Fonte restrita ou inviável permanece desabilitada para a finalidade incompatível. Se não houver modalidade gratuita compatível, registrar a inviabilidade e a decisão de escopo necessária.
- A liberação é por fonte e finalidade: G6/G7 precisam de suas próprias evidências, mesmo que o COTAHIST já esteja liberado.

**Evidência de conclusão:** catálogo datado, manifesto de corpus e declaração explícita do recorte/finalidade liberados. A evidência externa não pode ser substituída por configuração do operador ou sucesso de uma requisição HTTP.

## 5. G1 — Fechar identidade, revisões e contrato inicial

Referência: PRD §§7–9, 12–13 e 19; [issue 02](issues/02-domain-and-contracts.md).

**Resultado:** um modelo implementável para preservar fatos, identidade temporal, histórico de versões e os contratos definidos pelo PRD para o primeiro lançamento.

**Entrada:** baseline consolidada. Especificação e cenários sintéticos podem avançar imediatamente; homologação definitiva de chaves e campos depende do corpus de G0.

### Entregas

- **G1.1 — Contratos e granularidade:** definir rotas, DTOs, enums, filtros, ordenação, IDs, proveniência e OpenAPI de `0.0.1` conforme o PRD. Não há obrigação de compatibilidade com respostas ou IDs anteriores ao primeiro lançamento. Perfilar colisões das chaves candidatas, preservando `DISMES`, prazo de termo, sessão, mercado e componentes nulos conforme a fonte. Relação reconstruível com instrumento não compõe a identidade de um fato.
- **G1.2 — Identidade temporal:** especificar `security_id` qualificado, ISIN, ticker, mercado e distribuição; precedência, contradições, janelas, chegada fora de ordem, reutilização de ticker e resolução tardia. Não fabricar `security_id` usando ticker nem inventar vigência antiga a partir do cadastro atual.
- **G1.3 — Publicação e histórico:** separar artefato, publicação, ocorrência/observação, versão de fato e `data_revision`. Definir escopo completo de revisão, manifesto de participação, seleção de versões, conflitos, remoções e retorno à revisão anterior. O ponteiro de leitura não pode depender apenas da hora de download.
- **G1.4 — Inicialização limpa e evolução:** inicializar o schema `0.0.1` em base nova, sem importar registros, IDs, auditoria ou estado de ingestão anteriores. Validar migrations sequenciais desde banco vazio; não reescrever migrations aplicadas. Registrar decisões de domínio aceitas nos contextos/ADRs. A compatibilidade passa a ser garantida para dados/IDs publicados a partir de `0.0.1`.

### Critérios de saída

- Há resultados esperados para ticker reutilizado, identificador forte contraditório, cadastro tardio, vínculo ambíguo e ausência de cadastro histórico.
- Mesma chave e mesmo conteúdo compartilham fato/evidência conforme a política; conteúdo divergente preserva versões e conflito quando não há precedência oficial.
- A política cobre sobreposição anual/mensal/diária sem “último download vence”; ordem de member não se transforma em autoridade editorial.
- O manifesto da revisão declara os arquivos necessários e a precedência admitida; ordem de download ou quantidade de registros não prova completude nem autoridade.
- Identificadores públicos definidos em `0.0.1` permanecem resolvíveis durante as revisões subsequentes. Identidades físicas de versões não alteram silenciosamente o identificador público.
- Estados operacionais e campos públicos têm tipos e transições documentados no contrato inicial. Antes do lançamento, o contrato pode ser alinhado diretamente ao PRD sem obrigação de manter envelopes legados.

**Evidência de conclusão:** especificação de chaves/colisões e identidade temporal, cenários de aceitação, contrato inicial e migrations que inicializam o schema vazio. A homologação definitiva das chaves dependentes de conteúdo continua sujeita ao corpus permitido de G0.

## 6. G2 — Entregar aquisição, inventário e replay seguros

Referência: PRD §§10.1–10.4 e 18; [issue 03](issues/03-artifact-acquisition.md).

**Resultado:** qualquer entrada aceita pode ser localizada, verificada e reinterpretada sem novo download obrigatório e sem retenção integral em memória.

**Entrada:** contratos mínimos de artefato/member e limites de G1. Adaptadores externos exigem G0; testes locais e HTTP simulado não dependem de acesso à B3.

### Entregas

- **G2.1 — Transporte limitado:** download incremental para temporário, SHA-256 e tamanho incrementais, limites/timeouts e finalização atômica. Validar HTTPS, hosts, redirects e destinos de rede; homologar fluxos de duas etapas por canal. Remover tokens temporários de logs, erros e proveniência pública.
- **G2.2 — Raw e inventário:** persistir bytes imutáveis e manifesto; inventariar todos os members em ordem determinística com caminho aninhado, pai, índice, hashes e tamanhos. Entradas não selecionadas recebem motivo. Confinar importação local ao diretório permitido.
- **G2.3 — Extração e interfaces de leitura limitadas:** aplicar limites de bytes comprimidos/descomprimidos, razão de expansão, entradas e profundidade. Disponibilizar leitura incremental dos members aos parsers das famílias do recorte, sem acumular todos os conteúdos. Os contratos incluem limite de registro e de lote em linhas e bytes; G3/G6/G7 integram a interpretação tipada de cada formato. Parsing bloqueante tem capacidade limitada fora do I/O do Tokio.
- **G2.4 — Replay e recuperação da aquisição:** reutilizar raw sem apagar tentativas, detectar conteúdo já conhecido e recuperar interrupção entre download e registro do manifesto. Nenhum arquivo parcial vira entrada íntegra.

### Critérios de saída

- HTTP simulado cobre redirect permitido/proibido, destino inválido, token de dois passos, timeout, transferência interrompida, HTML de erro, tamanho excedido e ausência de validators HTTP.
- Arquivos com vários members, nomes repetidos em containers diferentes, path traversal, aninhamento excessivo e expansão acima do limite produzem inventário ou falha explícitos, sem omissão silenciosa.
- Replay com rede indisponível funciona a partir do raw preservado; checksum inválido e raw ausente permanecem falhas diagnosticáveis.
- Medição reproduzível mostra que download, inventário e extração respeitam os limites configurados. Os valores iniciais de orçamento são definidos a partir de corpus e hardware medidos.

**Evidência de conclusão:** fixtures com manifesto, testes determinísticos, trilha de falha/retomada e relatório de recursos da fase de aquisição. G2 encerra a primeira fase da ingestão; não comprova, sozinho, publicação de fatos.

## 7. G3 — Construir o núcleo histórico publicável

Referência: PRD §§5–10, 12 e 19; [issue 04](issues/04-historical-core.md).

**Resultado:** COTAHIST consultável com valores exatos, proveniência até registro e identidade sustentada pelo cadastro disponível, publicado em revisões consistentes e recuperáveis.

**Entrada:** G1, G2, corpus permitido e G5.1 operacional. Ingerir primeiro os cadastros disponíveis, preservando preços históricos válidos sem vínculo quando não houver evidência suficiente.

### Entregas

- **G3.1 — Schema e inicialização:** aplicar as migrations sequenciais em uma base nova e validar o estado vazio antes da ingestão. Não importar dados, IDs, auditoria ou progresso de ingestão do runtime anterior. Depois do lançamento, alterações estruturais seguem por migrations novas, sem reescrever migrations já aplicadas.
- **G3.2 — Interpretação e normalização:** streaming COTAHIST com largura de 245 bytes, encoding, header/trailer, decimais implícitos, sentinelas, moeda, `FATCOT` e `DISMES`. Homologar os adaptadores de cadastro utilizados, mantendo BVBG.028, Tradable Security List e InstrumentsConsolidated distintos.
- **G3.3 — Persistência, identidade e auditoria:** staging tipado e operações em lote; resolução temporal; fatos, alterações de campo, proveniência e contadores confirmados atomicamente no escopo do lote. Separar alteração de valor, observação equivalente e rebuild de identidade.
- **G3.4 — Revisão completa:** validar participação de todos os members obrigatórios e promover uma revisão por troca atômica de visibilidade. Manter a anterior servível; suportar revisão degradada apenas pela política explícita de rejeições locais. XML truncado ou controles estruturais inválidos não viram publicação completa.
- **G3.5 — Piloto e recuperação:** executar ingestão e replay do recorte, reconciliação contra o manifesto e resultados esperados, falhas injetadas e retorno à revisão anterior. Medir recursos antes de ampliar o backfill.

### Critérios de saída

- Repetição, retry e `force_reparse` não duplicam fatos, apagam auditoria ou geram alterações falsas. Mudança de um campo altera somente o conteúdo e os timestamps correspondentes, com evento old/new na mesma transação.
- Duas versões legítimas da mesma chave coexistem sem sobrescrita destrutiva; conteúdo inalterado reaproveita evidência conforme o contrato. IDs públicos introduzidos em `0.0.1` continuam resolvíveis.
- Reparse inalterado que cria uma nova observação de arquivo não oculta os fatos ligados ao arquivo anterior. Antes de ativar o novo filtro de leitura, reconciliar participação, contagens e hashes dos fatos selecionados.
- Falha no segundo member, após lotes válidos do primeiro, deixa a revisão anterior pública. Retry recupera o trabalho sem dupla auditoria; retorno à revisão anterior reproduz o conjunto esperado.
- Sobreposição diário/mensal/anual preserva todas as ocorrências de suporte, evita duplicação na consulta e torna divergências explícitas. Remoção de registro exige escopo completo e regra comprovada; ausência no acumulado corrente não elimina datas futuras.
- Linhas inválidas ficam em quarentena recuperável, com razão e localizador. Registros válidos e não resolvidos permanecem preservados. Rebuild temporal não muda valores, hashes de mercado ou timestamps de alteração de preço.
- Contadores confirmados fecham `detail_records_read = inserted + changed + unchanged + duplicate_in_scope + rejected`; ambiguidade e ausência de vínculo são dimensões adicionais.
- Todas as linhas válidas dos arquivos aceitos são preservadas, inclusive mercados e instrumentos fora do caso inicial de ações à vista. Não há classificação por sufixo de ticker, conversão histórica indiscriminada para BRL, preenchimento de lacunas ou ajuste retroativo.

**Evidência de conclusão:** inicialização reproduzível em banco vazio, manifesto da revisão, reconciliação de fatos/IDs/auditoria contra o corpus e o resultado esperado, cenários de falha e replay no PostgreSQL 16 e benchmark do piloto. Não se exige paridade com dados ou respostas legados.

## 8. G4 — Entregar consultas REST e MCP confiáveis

Referência: PRD §§13–16; [issue 05](issues/05-query-product.md).

**Resultado:** consumidores consultam o recorte com a mesma semântica em REST/MCP e conseguem identificar origem, revisão, incerteza e lacunas.

**Entrada:** revisão publicável de G3 e contratos de G1. Caracterização de v1 e extração dos serviços podem começar antes; aceite final depende do núcleo novo.

### Entregas

- **G4.1 — Serviços de consulta:** mover regras e SQL de leitura para serviços compartilhados. REST e MCP são adaptadores; consultas não ingerem, reprocessam ou resolvem relações pesadas em tempo de requisição.
- **G4.2 — Contratos e compatibilidade:** aplicar filtros de família, datas, mercado e identidade; `limit=100`, máximo 1000, `offset` e ordenação com desempate único. O contrato inicial é definido pelo PRD em `0.0.1`; não há compatibilidade obrigatória com respostas anteriores ao lançamento. Mudanças posteriores seguem a política de compatibilidade publicada.
- **G4.3 — Revisão, proveniência e cobertura:** indicar `data_revision`, proveniência por item e limitações essenciais de identidade/qualidade. Diferenciar ausência de negociação, arquivo não obtido, falha, quarentena e cobertura desconhecida. Paginação consistente fixa uma revisão por mecanismo compatível/aditivo.
- **G4.4 — Segurança e documentação:** separar leitura e administração antes de liberar a operação a consumidores; manter exceções públicas explícitas. Documentar origem, unidades, filtros, defaults, exemplos e erros em OpenAPI e DTOs, com README, documentação de consumidor e changelog coerentes.
- **G4.5 — MCP ponta a ponta:** testar inicialização e cliente reais com credencial PostgreSQL somente leitura, sem DDL, sync, replay, rebuild ou SQL arbitrário.

### Critérios de saída

- O mesmo filtro REST/MCP seleciona os mesmos fatos, precisão, identidade e proveniência na mesma revisão, respeitando os envelopes de cada transporte.
- Decimais são strings, datas ISO e timestamps UTC. Metadados adicionais obedecem `include_internal`; incerteza essencial não fica escondida nessa flag.
- Séries exigem desambiguação quando restam mercados/distribuições/prazos incompatíveis. O aviso de preços nominais COTAHIST sem ajustes aparece no payload, OpenAPI, MCP e documentação aplicáveis.
- Testes cobrem autenticação, autorização administrativa, 400/401/404/409/500 aplicáveis, paginação, filtros inválidos, limites e proveniência; o OpenAPI gerado corresponde às rotas reais.
- O namespace `/api/v1/b3` corresponde às rotas definidas pelo PRD. IDs e payloads anteriores não são migrados; após `0.0.1`, IDs expostos e sua semântica permanecem estáveis conforme a política publicada.
- Consulta durante promoção não mistura revisões; exportação/paginação mantém o conjunto prometido pelo contrato. `as_of` só é anunciado no horizonte de versões realmente capturado e testado.

**Evidência de conclusão:** comparação contratual com a baseline, testes REST/OpenAPI e MCP, exemplos de cobertura e documentação da finalidade autorizada. Endpoints de famílias ainda não habilitadas não sugerem dados disponíveis.

## 9. G5 — Operar manutenção contínua com recuperação

Referência: PRD §§10–11, 14 e 17–18; [issue 06](issues/06-operations.md).

**Resultado:** atualização regular recuperável, com estado durável e capacidade protegida contra consultas, backfill e rebuild.

**Entrada:** definições de escopo, estado e idempotência de G1. O desenvolvimento começa com G2/G3; somente os ensaios finais dependem do núcleo completo.

### G5.1 — Coordenação mínima para o novo pipeline

- API e CLI submetem comandos aos mesmos serviços e estados PostgreSQL. Jobs possuem claim durável, lease, heartbeat, geração de fencing, tentativas e cancelamento persistido.
- Migrations de coordenação e de suporte ao manifesto acompanham G2/G5.1; não aguardar as migrations de fatos de G3 para persistir o estado mínimo da execução.
- Separar aquisição da transação curta de promoção. Reter a conexão/transação que detém o lock durante toda a seção protegida; não manter transação aberta por horas de download.
- Proteger sobreposição real de fatos, inclusive COTAHIST diário/mensal/anual, com ordem estável de locks. Concorrência incompatível retorna `409`; repetição da mesma chave de idempotência e payload retorna o trabalho existente.
- Cancelamento em download, member, lote e publicação libera recursos e preserva confirmações anteriores sem publicar uma revisão incompleta.

**Saída para iniciar a integração de G3:** executor e estados duráveis implementados, com testes de dois processos sobre claim, exclusão, crash/expiração e fencing da confirmação. O aceite conjunto de G3/G5.1 repete esses cenários no pipeline real de fatos e publicação. Testar um helper isolado não comprova essa integração; lease sem validação de fencing na confirmação também não.

### G5.2 — Descoberta e calendário

- Scanner leve registra checagens conclusivas/inconclusivas e `b3_pending_updates`; não baixa/parsa arquivos pesados nem publica revisões durante descoberta.
- Manter promoção administrativa como política inicial. Automação de ingestão por fonte só é habilitada por política explícita após qualidade, recuperação e elegibilidade comprovadas.
- Usar `America/Sao_Paulo`, datas de mercado e calendário B3 versionado conforme atividade/fonte. Não recriar a tabela de calendário já existente; estendê-la apenas por migration necessária.
- Calendário ausente permanece desconhecido. Não preencher 1986–presente por regra de dias úteis ou feriados presumidos. Manutenção administrativa de referências passadas pode executar no fim de semana.
- Diferenciar saúde da descoberta agendada de execução manual, ausência de publicação esperada e falha de canal. Uma execução manual não renova artificialmente o indicador do agendador.

**Saída:** testes de fronteira UTC/São Paulo, feriados/sessões especiais com evidência, calendário ausente, validators HTTP ausentes, checagem inconclusiva e descoberta sem ingestão embutida.

### G5.3 — Recursos, diagnóstico e restauração

- Reservar conexões e concorrência para manutenção; limitar queries e classes de trabalho. Backfill/rebuild não esgotam capacidade nem impedem cancelamento.
- Expor saúde, cobertura, pendências, quarentena e estatísticas por run, arquivo, member e fase. Runs com resultados mistos mantêm estados dos filhos; falha não vira sucesso com zero registros.
- Permitir que o operador identifique o problema e solicite a recuperação autorizada pela API, sem depender de inspeção manual de SQL ou logs para decidir o próximo passo.
- Correlacionar logs por entidades operacionais sem credenciais/tokens. Documentar retries, limpeza idempotente, retenção e recuperação a partir de raw e banco.
- Executar build/smoke de API e worker em configuração controlada, além de ensaiar backup, restauração e replay.

**Critérios de saída do G5 completo:** concorrência de consulta/backfill/manutenção medida, cancelamento e fencing verificados, recuperação de falhas demonstrada, calendário da manutenção habilitada sustentado, lacunas históricas explícitas e restore reproduzível. Registrar hardware, corpus, configuração, tempos, RSS, conexões e WAL; fixar orçamentos com base nessas medições, sem publicar metas como resultados.

## 10. G6 — Homologar BVBG.086 e BVBG.186

Referência: PRD §§5.4, 6, 8 e 13; [issue 07](issues/07-price-reports.md).

**Resultado:** boletins complementares por edição, com contratos independentes e comparação de campos equivalentes explicitamente definida.

**Entrada:** plataforma comum G1–G5 comprovada e G0 para cada canal/finalidade. Parsers já existentes e a fixture 086 baseada no catálogo são preparação, não homologação dos arquivos distribuídos.

### Entregas e critérios de saída

1. Homologar estruturas ISO 20022, namespaces, paths completos, edições, referência e sessões com corpus oficial permitido; distinguir versões do catálogo e do formato recebido.
2. Persistir 086 e 186 em famílias, tabelas, DTOs e linhagens separados, reutilizando aquisição, revisões, quarentena, auditoria e coordenação comuns.
3. Validar chaves com `days_to_settlement`, mercados e demais qualificadores aplicáveis; não colapsar instrumentos/negociações distintos ou chamar qualquer preço final de fechamento oficial.
4. Especificar comparação tipada com unidades, regras de correspondência, registros não comparáveis e divergências. Ausência em uma família não dispara fallback para a outra.
5. Comprovar repetição, republicação, falha parcial, conflito e queries por edição nos mesmos padrões do núcleo; atualizar REST, MCP quando aplicável, OpenAPI e documentação.

**Evidência de conclusão:** manifesto do corpus por família, perfil de colisões, testes de ingestão/contrato e exemplos de comparação. G6 não converte os boletins em substitutos silenciosos do COTAHIST.

## 11. G7 — Homologar índices e carteiras

Referência: PRD §§5.5, 6, 8 e 13; [issue 08](issues/08-indices-and-portfolios.md).

**Resultado:** valores de índice e composições consultáveis no contexto temporal correto, com tipos e unidades preservados.

**Entrada:** plataforma comum G1–G5 comprovada e G0 específico. Pode avançar independentemente de G6.

### Entregas e critérios de saída

1. Homologar BVBG.087 por bloco, distinguindo índices, IOPV, referências e outros valores publicados. Cada tipo recebe DTO, unidade, chave e semântica adequados.
2. Homologar canais oficiais de composição: data-base, publicação conhecida, vigência e estágio `preview`/`effective`, além de paginação e horizonte demonstrado. Endpoint usado pelo site não é contrato operacional homologado por si só.
3. Separar carteira efetiva, prévias e fotografias diárias de pesos. Não usar composição atual para o passado nem fixar peso diário durante toda a vigência.
4. Preservar componentes sem vínculo resolvido; a chave da composição não depende de `instrument_id` reconstruível. Testar cadastro tardio sem duplicar ou apagar componentes.
5. Integrar revisões, conflitos, proveniência, cobertura e contratos. Documentar períodos indisponíveis sem prometer série histórica integral de carteiras ou índices.

**Evidência de conclusão:** corpus permitido, testes de blocos/unidades e transições de vigência/estágio, queries temporais e documentação por fonte. Novas estatísticas históricas de índices ou outras fontes candidatas permanecem fora do escopo até decisão explícita no PRD.

## 12. Aceite de lançamento e expansão

### 12.1 Entrega técnica de engenharia

G1, G2, G5.1 e partes de G3/G4 podem ser demonstrados com fixtures sintéticas/autorizadas em ambiente isolado. Essa entrega comprova software e recuperação no corpus de teste; não constitui lançamento de dados históricos para terceiros e não encerra a homologação de fonte pendente.

### 12.2 Primeiro lançamento histórico utilizável

Exige **G0–G5 concluídos para um recorte declarado**, incluindo os controles comuns de segurança, precisão, identidade, proveniência, revisão, recuperação e recursos. O manifesto do lançamento deve informar:

- Finalidade autorizada, fontes/canais habilitados, períodos e formatos efetivamente homologados.
- Todos os registros válidos preservados nos arquivos aceitos, mercados consultáveis, resolução de identidade e lacunas conhecidas.
- `data_revision`, versões de parser/regras, política de seleção e tratamento de rejeições/conflitos.
- Contrato REST/MCP disponível, limitações, configuração reproduzível e procedimento de recuperação.
- Evidências de upgrade, compatibilidade, concorrência, cancelamento, restore e recursos do recorte.

Não exige todo o histórico anunciado desde 1986, nem a conclusão simultânea de G6/G7. Também não permite omitir recuperação, autorização administrativa ou clareza de cobertura para antecipar a publicação.

### 12.3 Ampliar cobertura e completar famílias

Após o piloto, ampliar backfill por períodos controlados, com reconciliação, prioridade de manutenção e capacidade medida. A cada expansão, atualizar o manifesto de cobertura e revalidar colisões, formatos, moedas e comportamento de recursos encontrados no novo corpus.

G6/G7 completam a abrangência de famílias do PRD conforme seus próprios gates. Mesmo após esses marcos, “todas as famílias habilitadas” e “histórico integral de cada família” são afirmações diferentes. A disponibilidade real continua sendo demonstrada por cobertura.

## 13. Evidência e processo de encerramento

### 13.1 Matriz mínima de validação

A matriz resume as evidências por marco; os cenários obrigatórios da seção 18 do PRD continuam aplicáveis e não são substituídos por este resumo.

| Marco | Evidência que bloqueia seu encerramento se ausente |
| --- | --- |
| G0 | Elegibilidade por finalidade, condições sem custo e corpus com origem verificável |
| G1 | Chaves/colisões, identidade temporal, política de revisão, contrato 0.0.1 e migrations para banco vazio |
| G2 | Raw/inventário íntegros, limites medidos, HTTP simulado e replay após interrupção |
| G3 | Banco vazio inicializado; corpus/revisão reconciliados; IDs do novo contrato; rollback; contadores/auditoria; retry/reparse/quarentena |
| G4 | REST/OpenAPI do contrato 0.0.1; paginação por revisão; serviços compartilhados; MCP real com credencial restrita |
| G5 | Dois processos, sobreposição de escopos, crash/lease/fencing, cancelamento, calendário, prioridade, smoke e restore |
| G6 | Corpus 086/186, edições/chaves e comparação sem merge ou fallback |
| G7 | Blocos/unidades 087 e composições com vigência, estágio e cobertura comprovados |

### 13.2 Validação por entrega

Mudanças em Rust, dependências ou migrations executam, pelo serviço de desenvolvimento Compose:

```bash
docker compose run --rm cargo cargo fmt --all -- --check
docker compose run --rm cargo cargo clippy --all-targets --all-features -- -D warnings
docker compose run --rm cargo cargo test --all-targets --all-features
```

Os testes de persistência, migração, locks e concorrência usam **PostgreSQL 16 real**, com `DATABASE_URL` apontando para base de teste isolada. Ausência de banco deve falhar, não pular integração. Validar upgrades sem tocar uma base operacional. A CI deve manter os mesmos checks e garantias, conforme PRD §23.

Mudanças de runtime, configuração ou inicialização acrescentam build e smoke dos serviços reais `b3_api` e `b3_worker`, com aquisição externa desabilitada ou controlada antes de iniciá-los. O Compose não habilita descoberta nem canais por padrão; jobs de aquisição exigem comando administrativo e elegibilidade explícita. Não usar um worker contra fontes externas como teste determinístico nem remover volumes persistentes para obter uma base vazia.

Entregas de contrato incluem testes REST/OpenAPI, autenticação, filtros, precisão, proveniência, metadados internos e aviso COTAHIST. Alterações MCP incluem teste do transporte/processo quando a garantia depende dele. Atualizar OpenAPI, README, documentação de consumidor/operação e `frontend_api_changelog.md` no mesmo conjunto de mudanças aplicável.

Mudanças apenas documentais exigem consistência de links, caminhos e comandos; build de documentação somente quando houver gerador afetado. Elaborar este plano não executa os checks Rust acima nem altera o estado de implantação.

### 13.3 Registro de conclusão

Para concluir um marco ou subentrega, registrar na issue correspondente:

1. Escopo exato entregue e itens ainda pendentes, com referência ao PRD.
2. Commit de implementação, migrations, versões de parser/regras e corpus identificados por manifesto/hash.
3. Comandos executados, ambiente, resultados, falhas e checks não executados com motivo. Evidência de recurso inclui hardware e configuração.
4. Integridade do novo conjunto: corpus/hash, contagens, IDs publicados, auditoria e revisão ativa esperados/obtidos, além do caminho de recuperação. Migração de dados/IDs do runtime anterior não faz parte do escopo `0.0.1`.
5. Links para testes, documentação atualizada e relatórios, sem segredos ou cópias indevidas de dados brutos.

Relatórios intermediários podem ficar em `.scratch/<feature-slug>/`; decisões e evidências necessárias à manutenção devem ter referências duráveis na issue/documentação aceita. Não sobrescrever a baseline anterior para substituir uma falha por um resultado novo.

Uma subentrega concluída não encerra automaticamente o marco. A issue pode continuar `needs-info` enquanto resta dependência externa ou técnica, com o progresso registrado em `## Comments`. Atualizar status e índice juntos quando a prontidão mudar; criar novas issues a partir de `10` apenas se a decomposição justificar, sem duplicar 01–09.

## 14. Próximas entregas recomendadas

| Ordem | Entrega concreta | Trabalho rastreado | O que libera |
| --- | --- | --- | --- |
| 1 | Resolver evidência de elegibilidade por fonte, canal e finalidade; manter todos os canais sem evidência desabilitados | G0 / issue 01 | Aquisição e consumo externo autorizados para finalidades comprovadas |
| 2 | Adquirir e congelar o corpus oficial autorizado de 2025–2026, com corte, calendário, hashes e manifesto | G0/G3 / issues 01 e 04 | Homologação do recorte declarado e validação das natural keys |
| 3 | Reconciliar sobreposições, cadastros, conflitos e cobertura; repetir recuperação e publicação no corpus permitido | G1/G3 / issues 02 e 04 | Revisão histórica qualificável e restaurável |
| 4 | Medir no corpus permitido e no hardware-alvo; fixar budgets e fechar smoke, backup/restore e documentação operacional | G5 / issue 06 | Aceite operacional de W8 |
| 5 | Executar aceite integral, congelar manifesto de release e liberar consumidores somente com G0–G5 aprovados | W9 / plano 0.0.1 | Primeiro lançamento externo 0.0.1 |
| Depois | Planejar elegibilidade/corpora próprios e ativar 086/186, 087 e composições | G6/G7 / issues 07–08 | Expansão posterior ao primeiro lançamento |

Não há duração estimada enquanto G0 e o acesso ao corpus permanecerem pendentes. O benchmark sintético não substitui dimensionamento em corpus autorizado e hardware-alvo. G6/G7 são expansão posterior; não fazem parte do critério de conclusão de `0.0.1`.

## Comments

- 06/11/2026 — Marcos detalhados a partir do PRD v2, da base `fdc804f`, do backlog e das evidências da revisão. Nenhum marco foi declarado concluído e nenhuma capacidade proposta foi tratada como implementada. Alteração documental; testes Rust e homologações externas não foram executados nesta elaboração.
- 06/11/2026 — Implementação integrada registrada no commit `7e93b0d`; 65 testes passaram em PostgreSQL 16.15 isolado, além de build/smoke e benchmark sintético. As seções de execução foram atualizadas para a decisão posterior de base nova/contrato inicial, sem migração de registros ou IDs legados. G0 e W9 seguem bloqueados por evidência de elegibilidade, corpus oficial, cutoff e homologação de escala.
