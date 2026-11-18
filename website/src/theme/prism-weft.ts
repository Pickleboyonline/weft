// Prism grammar for weft, for code blocks on this site.
//
// The VS Code extension's TextMate grammar (editors/vscode) is the reference,
// and both follow the language reference (website/docs/reference/). This is a
// smaller approximation whose token names map to the same colors (see
// prismTheme in docusaurus.config.ts): declarations and the `!` effect marker
// as keywords, control flow in its own color, UI elements as tags, and CSS
// inside `style { … }` blocks.
import type * as PrismNamespace from 'prismjs';

const ident = String.raw`[a-z_][A-Za-z0-9_]*`;

const markupTags =
  'main|nav|section|header|footer|article|aside|div|form|input|textarea|select|option|label|button|checkbox|ul|ol|li|span|p|a|link|img|h[1-6]|strong|em|code|pre|br|hr';

export default function registerWeft(Prism: typeof PrismNamespace): void {
  // A string's ${...} interpolation is highlighted with the whole grammar, so
  // its `inside` is filled in once the grammar exists (end of this function).
  const stringInside: Record<string, PrismNamespace.GrammarValue> = {};

  const weft: PrismNamespace.Grammar = {
    'doc-comment': {pattern: /##.*/, greedy: true, alias: 'comment'},
    comment: {pattern: /#.*/, greedy: true},
    'triple-string': {pattern: /"""[\s\S]*?"""/, greedy: true, alias: 'string', inside: stringInside},
    string: {pattern: /"(?:\\.|[^"\\])*"/, greedy: true, inside: stringInside},
    // A component's scoped CSS: from `style {` to the `}` indented like it.
    'style-block': {
      pattern: /^([ \t]*)style\s*\{[\s\S]*?\n\1\}/m,
      greedy: true,
      inside: {
        'style-keyword': {pattern: /^[ \t]*style/, alias: 'keyword'},
        rest: Prism.languages.css,
      },
    },
    // ! <write items, reject [EmptyTitle], ..>
    'effect-row': {
      pattern: /!\s*<[^>]*>/,
      inside: {
        effect: /^!/,
        'effect-kind': /\b(?:read|write|reject|random|time|set|send|extern)\b/,
        punctuation: /[<>[\],]|\.\./,
        'class-name': /\b[A-Z]\w*\b/,
      },
    },
    declaration: {
      pattern: /\b(?:record|component|state|var|effect|handler|where|expect)\b/,
      alias: 'keyword',
    },
    // Contextual keywords: `view =` in a component, `key` before a record
    // field, `final` and `ctl` before an effect operation.
    contextual: {
      pattern: new RegExp(
        String.raw`(^[ \t]*)(?:view(?=\s*=)|key(?=\s+${ident}\s*:)|(?:final|ctl)(?=\s+${ident}!))`,
        'm',
      ),
      lookbehind: true,
      alias: 'keyword',
    },
    control: /\b(?:if|else|match|for|in|while|break|continue|return|then|reject|crash|with)\b/,
    logical: /\b(?:and|or|not)\b/,
    boolean: /\b(?:True|False)\b/,
    'constant-language': /\b(?:nothing|outlet)\b/,
    tag: new RegExp(String.raw`\b(?:${markupTags})\b(?=\s*[({])`),
    // A name followed by `!` is effectful: the bang gets its own token.
    function: {
      pattern: new RegExp(String.raw`\b${ident}!?(?=\s*\()`),
      inside: {effect: /!$/},
    },
    'effectful-name': {
      pattern: new RegExp(String.raw`\b${ident}!(?!=)`),
      alias: 'function',
      inside: {effect: /!$/},
    },
    // Record fields and named arguments: { draft: Text }, on_click: save
    property: new RegExp(String.raw`\b${ident}!?(?=\s*:(?!:))`),
    parameter: {
      pattern: /\|[^|\n]*\|/,
      inside: {punctuation: /\|/, ',': {pattern: /,/, alias: 'punctuation'}},
    },
    'class-name': /\b[A-Z][A-Za-z0-9_]*\b/,
    number: /\b\d[\d_]*(?:\.\d+)?(?:(?:ns|us|ms|s|min|h|d|w|B|KB|MB|GB|TB|f)\b|%)?/,
    effect: /=>/,
    operator: /->|\.\.[<=]|\.\.|==|!=|<=|>=|\/\/|[<>=+\-*/%?]/,
    punctuation: /[{}[\](),.:]/,
  };
  stringInside.interpolation = {
    pattern: /\$\{[^}]*\}/,
    // Prism's types can't express a grammar with both a custom token and `rest`.
    inside: {
      'interpolation-punctuation': {pattern: /^\$\{|\}$/, alias: 'punctuation'},
      rest: weft,
    } as PrismNamespace.Grammar,
  };
  Prism.languages.weft = weft;
}
