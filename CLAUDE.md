# weft

Side project: "JAX for general SWE". Logic is written in a restricted model with
explicit escape hatches. A compiler handles distribution, coordination, storage
and testing. The running example is a simple CRUD app.

The first scaffold (Python tracing front end, IR v0, deterministic simulator) is
in commit fb58534, kept for reference. The redesign moves the user-facing layer
up to a logical program plus placement/consistency annotations.

The current language proposal is [docs/language.md](docs/language.md). Its open
decisions (section 14) belong to the project owner: don't treat its
recommendations as settled. Concept notes on the physical layer:
[docs/jax-analogy.md](docs/jax-analogy.md) and
[docs/infrastructure-primitives.md](docs/infrastructure-primitives.md).

`examples/` holds weft projects written against that proposal, starting with
[examples/todo-lists](examples/todo-lists/README.md). Nothing compiles yet:
they're design artifacts, and each README lists the gaps it exposed.

[editors/vscode](editors/vscode/README.md) is a VS Code extension that
highlights `.weft` files and ```` ```weft ```` blocks in Markdown. When the
language changes, update its grammar and run `make vscode-test`: it fails if any
text in the examples or doc code blocks goes unhighlighted. `make` lists all
tasks.

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
