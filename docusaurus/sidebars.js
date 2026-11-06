/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  mainSidebar: [
    { type: 'doc', id: 'intro', label: 'Introdução' },
    {
      type: 'category',
      label: 'Primeiros passos',
      items: [
        'getting-started/installation',
        'getting-started/authentication',
        'getting-started/quickstart',
      ],
    },
    {
      type: 'category',
      label: 'Conceitos',
      items: [
        'concepts/data-model',
        'concepts/identity',
        'concepts/revisions',
        'concepts/provenance',
      ],
    },
    {
      type: 'category',
      label: 'Fontes B3',
      items: ['data-sources/overview', 'data-sources/cotahist', 'data-sources/registry'],
    },
    {
      type: 'category',
      label: 'API REST',
      items: [
        'api-endpoints/common-patterns',
        'api-endpoints/cotahist',
        'api-endpoints/instruments',
        'api-endpoints/operations',
        'api-endpoints/health',
      ],
    },
    {
      type: 'category',
      label: 'Operação',
      items: [
        'operations/overview',
        'operations/discovery',
        'operations/jobs',
        'operations/revisions',
        'operations/inspection',
      ],
    },
    {
      type: 'category',
      label: 'Schemas',
      items: [
        'schemas/common',
        'schemas/cotahist',
        'schemas/registry',
        'schemas/operations',
      ],
    },
    {
      type: 'category',
      label: 'Referência',
      items: ['reference/mcp', 'reference/errors', 'reference/glossary', 'reference/changelog'],
    },
  ],
};

module.exports = sidebars;
