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

Broken links fail the build. A link from a design note to a repo file
outside the site (`refs/`, `examples/`, `editors/`) can't resolve here, so
write it as a full GitHub URL, e.g.
`https://github.com/Pickleboyonline/weft/blob/main/refs/README.md`.

Every push to `main` builds the site and publishes it to
<https://pickleboyonline.github.io/weft/> (`.github/workflows/site.yml`).
