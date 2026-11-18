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
The one deliberate exception is distribution (placement, replication, client
copies of data, offline sync, consistency levels): it's an open question in
docs/language.md section 8. Don't add it to the reference, and don't make
guarantees that depend on the compiler inferring it. The fixed shape is one
server with a database, plus clients.

Key rules that are easy to get wrong:
- No `command`, `query`, `check`, `table`, `client`, `page` or `action`
  keywords. Remote functions are explicit values, `Query(|…| …)` and
  `Command(|…| …)`, with typed parameters and results; tables, checks,
  externs, jobs, deployments and simulations are values too
  (`items = Table(Item)`, `no_orphans = Check(|| …)`,
  `prod = Deployment { … }`). Folders are a convention, never semantics.
- Effects are Koka-style: inferred, never written. No `!` on names, no `=>`,
  no effect rows. Commands fail by returning `Err`; there's no `reject`.
- Table reads are language-integrated queries with a guarantee: one query per
  level of nesting, or a compile error. No silent in-memory fallback. Shared
  tables are server-only; clients reach them through queries and commands.
- Configuration is records and named arguments with ordinary values, never
  English-like phrases.
- Local bindings are `let x = …`, variables `let mut x = …`; shadowing is
  allowed. Plain `x = …` in a block is an assignment (to a `let mut` or a
  `state`); at the top level and for members it's a definition. No `var`.
- Components are Svelte-style: `state`, plain `=` assignment in handler
  functions, `view = …` markup, scoped `style { … }`. Beyond that, clients use
  general constructs, not special cases: postfix `.await` (no `then`/`else`),
  `task(key: …) { … }` with `defer`, `ref:` element handles, stateful
  functions, `Table(…, scope: Device | Memory)`, and platform modules (`http`,
  `ws`, `dom`, `browser`). Externs are for host libraries only.

`examples/` holds weft projects written in the designed syntax, starting with
[examples/todo-lists](examples/todo-lists/README.md). Nothing compiles yet:
they're design artifacts, and each README lists the gaps it exposed. Projects
follow the conventional layout on the reference's Projects page.

[tree-sitter-weft](tree-sitter-weft/README.md) is the grammar's executable
form and `crates/` is the checker (`weft check`, `weft explain effects|api`,
`weft explain <code>`, `weft fmt`, `weft doctest`). It parses and checks; it never generates code. When the
language changes, change the reference, then `grammar.js` (`make grammar`
fails on any ambiguity: resolve it explicitly, never with a declared
conflict, and record it in its README), then the checker. `make test` runs
everything: the checker's tests, the example project, every ```` ```weft ````
block in the docs (lines marked `// error:` must fail; all others must
pass), `weft fmt --check` on the example, and the highlighting tests. A new
diagnostic code needs an entry in `crates/weft-check/src/explain.rs`.

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
