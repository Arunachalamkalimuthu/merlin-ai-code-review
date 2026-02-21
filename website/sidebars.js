// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  docsSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Getting Started',
      collapsed: false,
      items: [
        'getting-started/installation',
        'getting-started/github-actions',
        'getting-started/gitlab-ci',
        'getting-started/other-platforms',
      ],
    },
    {
      type: 'category',
      label: 'Configuration',
      items: [
        'configuration/merlin-toml',
        'configuration/ai-providers',
        'configuration/environment-variables',
      ],
    },
    {
      type: 'category',
      label: 'Slash Commands',
      items: [
        'slash-commands/overview',
        'slash-commands/review',
        'slash-commands/spec',
        'slash-commands/describe',
        'slash-commands/ask',
        'slash-commands/improve',
        'slash-commands/security',
        'slash-commands/other-commands',
      ],
    },
    {
      type: 'category',
      label: 'RAG',
      items: [
        'rag/overview',
        'rag/embedders',
        'rag/vector-stores',
        'rag/ci-caching',
      ],
    },
    {
      type: 'category',
      label: 'Agent',
      items: [
        'agent/overview',
        'agent/slack',
        'agent/discord',
      ],
    },
    {
      type: 'category',
      label: 'Bot Mode',
      items: [
        'bot-mode/overview',
      ],
    },
    'contributing',
  ],
};

module.exports = sidebars;
