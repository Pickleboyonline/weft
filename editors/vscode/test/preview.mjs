// Renders .weft files in Dark+ and Light+ to out/preview.html, using VS Code's
// own themes, to eyeball the highlighting.
//
//   npm run preview               every .weft file in the repo
//   npm run preview -- app.weft   only files whose path contains "app.weft"
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { highlighter, weftFiles, readFileSync, relative, repoRoot, extRoot } from "./shared.mjs";

const hl = await highlighter();
const filters = process.argv.slice(2);
const files = weftFiles().filter((p) => !filters.length || filters.some((f) => p.includes(f)));

const sections = files
  .map((p) => {
    const code = readFileSync(p, "utf8");
    const dark = hl.codeToHtml(code, { lang: "weft", theme: "dark-plus" });
    const light = hl.codeToHtml(code, { lang: "weft", theme: "light-plus" });
    return `<h2>${relative(repoRoot, p)}</h2><div class="pair">${dark}${light}</div>`;
  })
  .join("\n");

const html = `<!doctype html>
<meta charset="utf-8">
<title>weft highlighting preview</title>
<style>
  body { font: 14px system-ui, sans-serif; margin: 24px; background: #888; }
  h2 { font-size: 14px; color: #fff; margin: 24px 0 8px; }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  pre { margin: 0; padding: 12px; border-radius: 6px; overflow-x: auto; font: 12px/1.45 ui-monospace, Menlo, monospace; }
</style>
<h1 style="color:#fff;font-size:16px">weft: Dark+ (left), Light+ (right)</h1>
${sections}
`;

const outDir = join(extRoot, "out");
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "preview.html"), html);
console.log(`wrote ${join(outDir, "preview.html")} (${files.length} files)`);
