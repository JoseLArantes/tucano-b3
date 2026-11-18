/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  mainSidebar: [
    { type: 'doc', id: 'intro', label: 'Introdução' },
    { type: 'doc', id: 'api-endpoints/index-metrics', label: 'Índices e CSV' },
    { type: 'doc', id: 'schemas/index-metrics', label: 'Schema dos índices' },
    { type: 'doc', id: 'operations/index-ingestion', label: 'Operação de índices' },
    {
      type: 'category',
      label: 'Ações e units',
      items: [
        'api-endpoints/equities',
        'schemas/equities',
        'operations/equity-data',
      ],
    },
  ],
};

module.exports = sidebars;
