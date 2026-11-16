# Documentação do Tucano B3

Este diretório contém o site Docusaurus para consumidores da API, operadores e pessoas que contribuem com o Tucano B3. As páginas descrevem o comportamento implementado e verificado; propósito e escopo do produto estão em `PRODUCT.md` na raiz do repositório.

## Desenvolvimento local

Requer Node.js 20 ou superior. A partir da raiz do repositório:

    npm --prefix docusaurus ci
    npm --prefix docusaurus start

O site local fica em http://localhost:3000. Para gerar e conferir a versão estática:

    npm --prefix docusaurus run build
    npm --prefix docusaurus run serve

O resultado do build fica em docusaurus/build/.

## Publicação

O workflow .github/workflows/deploy-docs.yml publica o build no GitHub Pages após push em main com mudanças no diretório docusaurus ou no próprio workflow. A URL configurada para este repositório é https://joselarantes.github.io/b3-tucano/.

## Organização

- docs/api-endpoints: métodos, filtros, autenticação, respostas e exemplos de ações e units;
- docs/operations: cobertura e atualização do dataset de ações verificado;
- docs/schemas: campos e formatos dos contratos de listagem e preço;
- docusaurus.config.js e sidebars.js: configuração e navegação;
- src/ e static/: página inicial, estilos e marca do site.

## Regras editoriais

- Escreva documentação pública em português brasileiro e preserve os identificadores técnicos.
- Use o OpenAPI servido pelo binário em /openapi.json como referência de contrato legível por máquina. A rota /docs do serviço serve a Swagger UI interativa ligada a esse documento; ela não substitui este site nem as permissões Bearer das operações.
- Descreva todos os métodos, parâmetros, limites, permissões, respostas e campos relevantes nas páginas de endpoint e schema.
- Mantenha cada afirmação alinhada a código, testes e estado publicado. Escopo em `PRODUCT.md` não significa, por si só, que uma fonte esteja habilitada ou disponível ao consumidor.
- Ao alterar contrato, atualize a descrição Utoipa/OpenAPI e as páginas correspondentes de endpoint e schema no mesmo trabalho.
- Antes de entregar mudanças no site, execute npm --prefix docusaurus run build e corrija links internos quebrados.
