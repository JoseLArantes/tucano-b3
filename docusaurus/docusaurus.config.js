// @ts-check
const lightCodeTheme = require('prism-react-renderer').themes.github;
const darkCodeTheme = require('prism-react-renderer').themes.dracula;

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Tucano B3 Docs',
  tagline: 'Histórico oficial da B3 com identidade, precisão e proveniência',
  favicon: 'img/tucano-b3-mark.svg',
  url: 'https://joselarantes.github.io',
  baseUrl: '/b3-tucano/',
  organizationName: 'JoseLArantes',
  projectName: 'b3-tucano',
  onBrokenLinks: 'throw',
  i18n: {
    defaultLocale: 'pt-BR',
    locales: ['pt-BR'],
  },
  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          editUrl: 'https://github.com/JoseLArantes/b3-tucano/tree/main/docusaurus/',
          routeBasePath: 'docs',
        },
        blog: false,
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
      },
    ],
  ],
  themes: [
    [
      require.resolve('@easyops-cn/docusaurus-search-local'),
      {
        hashed: true,
        indexDocs: true,
        indexBlog: false,
        indexPages: true,
        searchResultLimits: 8,
        searchResultContextMaxLength: 90,
      },
    ],
  ],
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },
  themeConfig: {
    image: 'img/tucano-b3-mark.svg',
    navbar: {
      title: 'Tucano B3',
      logo: {
        alt: 'Marca do Tucano B3',
        src: 'img/tucano-b3-mark.svg',
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'mainSidebar',
          position: 'left',
          label: 'Documentação',
        },
        {
          href: 'https://github.com/JoseLArantes/b3-tucano',
          label: 'Código',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Começar',
          items: [
            { label: 'Introdução', to: '/docs/intro' },
            { label: 'Consulta de ações e units', to: '/docs/api-endpoints/equities' },
            { label: 'Cobertura verificada', to: '/docs/operations/equity-data' },
          ],
        },
        {
          title: 'Referência',
          items: [
            { label: 'Endpoints', to: '/docs/api-endpoints/equities' },
            { label: 'Schemas', to: '/docs/schemas/equities' },
            { label: 'Operação', to: '/docs/operations/equity-data' },
          ],
        },
        {
          title: 'Projeto',
          items: [
            { label: 'GitHub', href: 'https://github.com/JoseLArantes/b3-tucano' },
            { label: 'README do projeto', href: 'https://github.com/JoseLArantes/b3-tucano#readme' },
          ],
        },
      ],
      copyright: 'Tucano B3 · Documentação técnica em português brasileiro',
    },
    prism: {
      theme: lightCodeTheme,
      darkTheme: darkCodeTheme,
      additionalLanguages: ['bash', 'json', 'rust'],
    },
    colorMode: {
      defaultMode: 'light',
      disableSwitch: false,
      respectPrefersColorScheme: true,
    },
  },
};

module.exports = config;
