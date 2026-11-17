/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  mainSidebar: [
    { type: 'doc', id: 'intro', label: 'Introdução' },
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
