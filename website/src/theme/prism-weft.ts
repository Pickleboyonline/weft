// Prism grammar for weft, for code blocks on this site.
//
// The VS Code extension's TextMate grammar (editors/vscode) is the reference.
// This is a smaller approximation whose token names map to the same colors
// (see prismTheme in docusaurus.config.ts): declarations and the `!` effect
// marker as keywords, control flow in its own color, UI elements as tags.
import type * as PrismNamespace from 'prismjs';

const ident = String.raw`[a-z_][A-Za-z0-9_]*`;

const markupTags =
  'main|nav|section|header|footer|article|aside|div|form|input|textarea|select|option|label|button|checkbox|ul|ol|li|span|p|a|link|img|h[1-6]|strong|em|code|pre|br|hr';

export default function registerWeft(Prism: typeof PrismNamespace): void {
  // A string's ${...} interpolation is highlighted with the whole grammar, so
  // its `inside` is filled in once the grammar exists (end of this function).
  const stringInside: Record<string, PrismNamespace.GrammarValue> = {};

  const weft: PrismNamespace.Grammar = {
    comment: {pattern: /#.*/, greedy: true},
    string: {pattern: /"(?:\\.|[^"\\])*"/, greedy: true, inside: stringInside},
    // ! <write todos, reject [EmptyTitle]>
    'effect-row': {
      pattern: /!\s*<[^>]*>/,
      inside: {
        effect: /^!/,
        'effect-kind': /\b(?:read|write|reject|random|time)\b/,
        punctuation: /[<>[\],]/,
        'class-name': /\b[A-Z]\w*\b/,
      },
    },
    declaration: {
      pattern: /\b(?:module|import|record|table|command|query|view|client|component|page|state|action|check|effect|handler|extern|fun|ctl|final|deploy|simulate|machine|place|consistency|durability|budget|pin|sync|needs|sim)\b/,
      alias: 'keyword',
    },
    control: /\b(?:if|else|match|for|in|key|do|reject)\b/,
    logical: /\b(?:and|or|not)\b/,
    boolean: /\b(?:True|False)\b/,
    'constant-language': /\b(?:none|nothing|outlet)\b/,
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
    // Record fields and named arguments: { draft: Text }, on_click: Select(id)
    property: new RegExp(String.raw`\b${ident}(?=\s*:(?!:))`),
    parameter: {
      pattern: /\|[^|\n]*\|/,
      inside: {punctuation: /\|/, ',': {pattern: /,/, alias: 'punctuation'}},
    },
    'class-name': /\b[A-Z][A-Za-z0-9_]*\b/,
    number: /\$?\b\d[\d_]*(?:\.\d+)?(?:ns|us|ms|s|m|h|d|%|KB|MB|GB|TB)?(?:\/(?:month|week|day|hour|year))?/,
    effect: /=>/,
    operator: /->|\.\.|==|!=|<=|>=|[<>=+\-*/%?]/,
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
