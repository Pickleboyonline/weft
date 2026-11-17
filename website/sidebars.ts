import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// The structure follows docs/docs-plan.md. Sections that aren't written yet
// (tutorial, how-to guides, concepts, reference, cheatsheets) are listed on the
// status page rather than shown as empty pages.
const sidebars: SidebarsConfig = {
  docs: [
    {
      type: 'category',
      label: 'Start',
      collapsible: false,
      items: ['big-idea', 'promise', 'at-a-glance', 'why-not'],
    },
    'status',
  ],
};

export default sidebars;
