# weft website

The documentation site, built with [Docusaurus](https://docusaurus.io/). From
the repo root:

```bash
make site          # dev server at http://localhost:3000, reloads on edit
make site-build    # static build into website/build
```

## Where things live

| Path | What |
|---|---|
| `docs/` | The user-facing docs. Structure per [docs/docs-plan.md](../docs/docs-plan.md). |
| `../docs/` | The repo's design notes, served under `/design` without copying them. |
| `sidebars.ts`, `sidebars-design.ts` | Page order for each section. New design notes must be added here. |
| `src/pages/index.tsx` | The home page. |
| `src/theme/prism-weft.ts` | Syntax highlighting for `weft` code blocks. A smaller version of the VS Code grammar in `editors/vscode`; keep the two in step. |
| `static/img/` | Logos and icons, copied from `branding/assets/`. |

Pages are plain Markdown (`.md`, CommonMark). Use `.mdx` only where a page
needs React components.

Links from the design notes to repo files outside the site (`refs/`,
`examples/`) can't resolve here, so the build reports broken links as
warnings, not errors. Check the warnings after adding a page.
