# weft

Side project: "JAX for general SWE". Logic is written in a restricted model with
explicit escape hatches. A compiler handles distribution, coordination, storage
and testing. The running example is a simple CRUD app.

The first scaffold (Python tracing front end, IR v0, deterministic simulator) is
in commit fb58534, kept for reference. The redesign moves the user-facing layer
up to a logical program plus placement/consistency annotations.

**The language reference is the specification:** `website/docs/reference/`
(18 pages, on the docs site under Reference). Any syntax in examples, docs or
grammars must match it; change the reference first.
[docs/language.md](docs/language.md) explains *why* the language looks as it
does. Its open decisions (section 8) belong to the project owner: don't treat
them as settled. Concept notes on the physical layer:
[docs/jax-analogy.md](docs/jax-analogy.md) and
[docs/infrastructure-primitives.md](docs/infrastructure-primitives.md). How
checks could be verified (simulation, model checking, proof):
[docs/testing-and-proof.md](docs/testing-and-proof.md).

Design the whole language at once: don't split features into "v1" and "later".

Key rules that are easy to get wrong:
- No `command`, `query`, `check`, `table`, `client`, `page` or `action`
  keywords. A function's role comes from its effects and folder; tables,
  externs, jobs, deployments and simulations are values (`items = Table(Item)`,
  `prod = Deployment { … }`).
- Configuration is records and named arguments with ordinary values, never
  English-like phrases.
- Components are Svelte-style: `state`, plain `=` assignment in handler
  functions, `view = …` markup, scoped `style { … }`.

`examples/` holds weft projects written in the designed syntax, starting with
[examples/todo-lists](examples/todo-lists/README.md). Nothing compiles yet:
they're design artifacts, and each README lists the gaps it exposed. Projects
follow the conventional layout on the reference's Projects page.

[editors/vscode](editors/vscode/README.md) is a VS Code extension that
highlights `.weft` files and ```` ```weft ```` blocks in Markdown. When the
language changes, update its grammar and run `make vscode-test`: it fails if any
text in the examples or doc code blocks goes unhighlighted. `make` lists all
tasks.

[website/](website/README.md) is the Docusaurus documentation site (`make site`
runs it). Its user-facing pages are in `website/docs/`, structured per
[docs/docs-plan.md](docs/docs-plan.md); the repo's `docs/` folder is served
as-is under "Design notes". Every page says weft is a design: never describe a
planned feature as if it exists. `website/src/theme/prism-weft.ts` is a smaller
copy of the VS Code grammar for the site; keep the two in step.

## Reference code

`refs/` holds read-only clones of related projects (Hydro, JAX, Triton, Wing,
Encore, DBOS, TigerBeetle, Maelstrom, Roc, Koka, Ur/Web, ScalaLoci, Electric,
Verdi). [refs/README.md](refs/README.md) says why each matters and where to
start reading. Papers are in [docs/reading-list.md](docs/reading-list.md).

- Never edit files under `refs/`; `refs/sync.sh` resets them.
- They're git-ignored, so project-wide searches skip them. Search them by path,
  e.g. `rg pattern refs/jax`.
- Add a repo by editing `refs/repos.txt` and running `refs/sync.sh <name>`.
- Restate (BSL), Convex (FSL) and Electric (commercial) are source-available:
  read, don't copy.
