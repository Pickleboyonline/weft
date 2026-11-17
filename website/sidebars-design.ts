import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// Pages from the repo's docs/ folder. Add new design notes here.
const sidebars: SidebarsConfig = {
  design: [
    'language',
    {
      type: 'category',
      label: 'Concepts',
      collapsible: false,
      items: ['jax-analogy', 'infrastructure-primitives'],
    },
    {
      type: 'category',
      label: 'Research',
      collapsible: false,
      items: ['prior-art', 'reading-list', 'docs-plan'],
    },
  ],
};

export default sidebars;
