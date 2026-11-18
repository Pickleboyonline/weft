import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// The structure follows docs/docs-plan.md. Sections that aren't written yet
// (tutorial, how-to guides, concepts, cheatsheets) are listed on the status
// page rather than shown as empty pages.
const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Start',
      collapsible: false,
      items: ['big-idea', 'promise', 'at-a-glance', 'why-not'],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsible: true,
      items: [
        'reference/overview',
        'reference/lexical',
        'reference/types',
        'reference/bindings',
        'reference/expressions',
        'reference/control-flow',
        'reference/functions',
        'reference/effects',
        'reference/data',
        'reference/commands-and-queries',
        'reference/ui',
        'reference/checks-and-tests',
        'reference/resources',
        'reference/deployment',
        'reference/projects',
        'reference/stdlib',
        'reference/runtime-errors',
        'reference/grammar',
      ],
    },
    'status',
  ],
};

export default sidebars;
