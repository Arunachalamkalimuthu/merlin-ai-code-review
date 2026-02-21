// @ts-check
const { themes: prismThemes } = require('prism-react-renderer');

const GITHUB_REPO = 'https://github.com/Arunachalamkalimuthu/merlin-ai-code-review';

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'Merlin',
  tagline: 'Self-hosted AI code review — open-source, bring-your-own-key',
  favicon: 'img/favicon.ico',

  url: 'https://arunachalamkalimuthu.github.io',
  baseUrl: '/merlin-ai-code-review/',

  organizationName: 'Arunachalamkalimuthu',
  projectName: 'merlin-ai-code-review',

  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'warn',

  i18n: { defaultLocale: 'en', locales: ['en'] },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: './sidebars.js',
          editUrl: `${GITHUB_REPO}/tree/main/website/`,
          showLastUpdateTime: true,
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      colorMode: {
        defaultMode: 'dark',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },

      navbar: {
        title: 'Merlin',
        logo: { alt: 'Merlin', src: 'img/logo.svg' },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'docsSidebar',
            position: 'left',
            label: 'Docs',
          },
          {
            to: '/docs/slash-commands/overview',
            label: 'Commands',
            position: 'left',
          },
          {
            to: '/docs/rag/overview',
            label: 'RAG',
            position: 'left',
          },
          {
            to: '/docs/agent/overview',
            label: 'Agent',
            position: 'left',
          },
          {
            href: GITHUB_REPO,
            label: 'GitHub',
            position: 'right',
          },
        ],
      },

      footer: {
        style: 'dark',
        links: [
          {
            title: 'Getting started',
            items: [
              { label: 'Installation', to: '/docs/getting-started/installation' },
              { label: 'GitHub Actions', to: '/docs/getting-started/github-actions' },
              { label: 'GitLab CI', to: '/docs/getting-started/gitlab-ci' },
              { label: 'Other platforms', to: '/docs/getting-started/other-platforms' },
            ],
          },
          {
            title: 'Reference',
            items: [
              { label: 'Configuration', to: '/docs/configuration/merlin-toml' },
              { label: 'AI providers', to: '/docs/configuration/ai-providers' },
              { label: 'Slash commands', to: '/docs/slash-commands/overview' },
              { label: 'Environment variables', to: '/docs/configuration/environment-variables' },
            ],
          },
          {
            title: 'Community',
            items: [
              { label: 'GitHub', href: GITHUB_REPO },
              { label: 'Issues', href: `${GITHUB_REPO}/issues` },
              { label: 'Contributing', to: '/docs/contributing' },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Merlin. MIT License.`,
      },

      prism: {
        theme: prismThemes.github,
        darkTheme: prismThemes.dracula,
        additionalLanguages: ['bash', 'toml', 'yaml', 'rust', 'json'],
      },

      algolia: undefined, // add Algolia DocSearch credentials here when ready
    }),
};

module.exports = config;
