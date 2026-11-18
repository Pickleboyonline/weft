import {themes as prismThemes} from 'prism-react-renderer';
import type {PrismTheme} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

// Colors for weft's own token types (src/theme/prism-weft.ts), matching the
// VS Code extension under Dark+ and Light+.
function withWeftTokens(base: PrismTheme, c: Record<string, string>): PrismTheme {
  return {
    ...base,
    styles: [
      ...base.styles,
      {types: ['control'], style: {color: c.control}},
      {types: ['logical', 'boolean', 'constant-language'], style: {color: c.keyword}},
      {types: ['tag'], style: {color: c.tag}},
      {types: ['property', 'parameter'], style: {color: c.property}},
    ],
  };
}

const lightCode = withWeftTokens(prismThemes.vsLight, {
  control: '#AF00DB',
  keyword: '#0000FF',
  tag: '#800000',
  property: '#001080',
});
const darkCode = withWeftTokens(prismThemes.vsDark, {
  control: '#C586C0',
  keyword: '#569CD6',
  tag: '#569CD6',
  property: '#9CDCFE',
});

const config: Config = {
  title: 'weft',
  tagline: 'One program. Woven together.',
  favicon: 'img/weft-avatar.svg',

  future: {
    v4: true,
  },

  // Deployed to GitHub Pages by .github/workflows/site.yml on every push to
  // main. Run locally with `make site`.
  url: 'https://pickleboyonline.github.io',
  baseUrl: '/weft/',
  organizationName: 'Pickleboyonline',
  projectName: 'weft',

  // Broken links fail the build. The design notes come straight from the
  // repo's docs/ folder, so a link from them to a repo file outside the site
  // (refs/, examples/, editors/) must be a full GitHub URL.
  onBrokenLinks: 'throw',

  markdown: {
    // .md files are plain CommonMark (the repo docs aren't written for MDX);
    // .mdx files get MDX.
    format: 'detect',
    hooks: {
      onBrokenMarkdownLinks: 'throw',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        // The working design notes in the repo's docs/ folder, served as-is.
        id: 'design',
        path: '../docs',
        routeBasePath: 'design',
        sidebarPath: './sidebars-design.ts',
      },
    ],
  ],

  themeConfig: {
    colorMode: {
      respectPrefersColorScheme: true,
    },
    navbar: {
      logo: {
        alt: 'weft',
        src: 'img/weft-logo-light.svg',
        srcDark: 'img/weft-logo-dark.svg',
      },
      items: [
        {type: 'docSidebar', sidebarId: 'docs', position: 'left', label: 'Docs'},
        {
          type: 'docSidebar',
          sidebarId: 'design',
          docsPluginId: 'design',
          position: 'left',
          label: 'Design notes',
        },
        {
          href: 'https://github.com/Pickleboyonline/weft',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Start',
          items: [
            {label: 'The big idea', to: '/docs/'},
            {label: 'The promise and its limits', to: '/docs/promise'},
            {label: 'Weft at a glance', to: '/docs/at-a-glance'},
          ],
        },
        {
          title: 'Design notes',
          items: [
            {label: 'Language proposal', to: '/design/language'},
            {label: 'Prior art', to: '/design/prior-art'},
            {label: 'Reading list', to: '/design/reading-list'},
          ],
        },
      ],
      copyright: 'weft is a design in progress. Nothing here compiles yet.',
    },
    prism: {
      theme: lightCode,
      darkTheme: darkCode,
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
