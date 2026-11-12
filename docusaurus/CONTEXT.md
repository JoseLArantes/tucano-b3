# Contexto da documentação

Este contexto cobre o site Docusaurus, os guias públicos da API, os schemas narrativos e a consistência com o contrato Utoipa.

## Escopo

- docusaurus/docs/
- docusaurus/sidebars.js
- docusaurus/src/ e docusaurus/static/
- docusaurus/docusaurus.config.js e docusaurus/package.json
- README e documentação de consumo que apontam para o site

## Fontes da verdade

- O comportamento implementado, os testes e o OpenAPI gerado descrevem o contrato atual.
- PRODUCT.md define propósito, escopo e princípios do produto; não substitui o contrato publicado nem comprova por si só que uma capacidade esteja habilitada.
- docs/domain/CONTEXT.md define vocabulário do domínio.
- docs/consumer_api.md e a documentação operacional na raiz permanecem referências textuais do projeto; o Docusaurus organiza os guias para consumidores e operadores.

## Regras de documentação

- Cada operação publicada tem documentação OpenAPI detalhada de origem, semântica, parâmetros, defaults, exemplos, autorização, idempotência e respostas aplicáveis.
- Páginas Docusaurus explicam o mesmo contrato com tabelas de parâmetros e campos, exemplos completos, comportamento de status e limitações de uso.
- Não substitua contratos tipados por JSON arbitrário nem descreva recursos do roadmap como disponíveis.
- O guia COTAHIST deve preservar em destaque que os valores são nominais brutos, na moeda e forma de cotação da época, sem ajuste retroativo por inflação ou eventos corporativos.
- Rotas, schemas, exemplos, README e OpenAPI devem concordar. Mudança de contrato requer atualização simultânea das descrições de endpoint e schema.
- Execute o build de produção do Docusaurus depois de qualquer alteração de conteúdo ou configuração.

## Vocabulário

- **OpenAPI gerado**: documento JSON do binário em /openapi.json, que descreve o contrato legível por máquina.
- **Site Docusaurus**: guias narrativos com endereço de publicação https://joselarantes.github.io/b3-tucano/.
- **Swagger UI da API**: interface interativa do serviço em /docs, ligada ao OpenAPI gerado e servido em /openapi.json; chamadas continuam sujeitas às permissões da operação. Não é a documentação Docusaurus.
- **Comportamento implementado**: comportamento apoiado por código e testes no estado atual, sem inferência a partir dos requisitos futuros.
