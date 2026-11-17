// Loads the weft grammars into Shiki, which tokenizes with vscode-textmate
// (the engine VS Code itself uses) and VS Code's own themes.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { createHighlighter } from "shiki";

export const here = dirname(fileURLToPath(import.meta.url));
export const extRoot = join(here, "..");
export const repoRoot = join(extRoot, "..", "..");

const load = (p) => JSON.parse(readFileSync(join(extRoot, p), "utf8"));

export const weftLang = { ...load("syntaxes/weft.tmLanguage.json"), name: "weft" };
const mdInjection = {
  ...load("syntaxes/weft.markdown.tmLanguage.json"),
  name: "weft-markdown-injection",
  injectTo: ["text.html.markdown"],
  embeddedLangs: ["weft"],
};

export async function highlighter() {
  return createHighlighter({
    themes: ["dark-plus", "light-plus"],
    langs: ["markdown", weftLang, mdInjection],
  });
}

// One entry per raw token: its text and its full scope stack.
export function rawTokens(hl, code, lang = "weft") {
  const lines = hl.codeToTokensBase(code, { lang, theme: "dark-plus", includeExplanation: true });
  const out = [];
  lines.forEach((line, lineNo) => {
    for (const tok of line) {
      for (const ex of tok.explanation ?? []) {
        out.push({ line: lineNo + 1, text: ex.content, scopes: ex.scopes.map((s) => s.scopeName) });
      }
    }
  });
  return out;
}

export function weftFiles(dir = repoRoot) {
  const found = [];
  for (const name of readdirSync(dir)) {
    if (["refs", "node_modules", ".git"].includes(name)) continue;
    const p = join(dir, name);
    if (statSync(p).isDirectory()) found.push(...weftFiles(p));
    else if (name.endsWith(".weft")) found.push(p);
  }
  return found;
}

// ```weft blocks from the Markdown docs, so the grammar is checked against the spec too.
export function docBlocks() {
  const blocks = [];
  const dirs = [join(repoRoot, "docs"), join(repoRoot, "examples")];
  const walk = (d) => {
    for (const name of readdirSync(d)) {
      const p = join(d, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (name.endsWith(".md")) {
        const text = readFileSync(p, "utf8");
        for (const m of text.matchAll(/^```weft\n([\s\S]*?)^```/gm)) {
          blocks.push({ file: relative(repoRoot, p), code: m[1] });
        }
      }
    }
  };
  dirs.forEach(walk);
  return blocks;
}

export { readFileSync, relative };
