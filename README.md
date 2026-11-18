<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="website/static/img/weft-logo-dark.svg">
    <img alt="weft" src="website/static/img/weft-logo-light.svg" height="64">
  </picture>
</p>

<p align="center"><b>One program. Woven together.</b></p>

<p align="center"><a href="https://pickleboyonline.github.io/weft/">Documentation</a></p>

---

**weft is a language design, not a working language yet.** Nothing compiles
to a running app. What exists today is the specification, a grammar and a
checker.

The idea is that you write the system once, as if it ran on one machine, and
that program is the specification. weft builds the running version (a
server with its database, browser clients, and the typed calls between
them) and checks it against what you wrote. It's "JAX for general software
engineering": logic is written in a restricted model with explicit escape
hatches, and a compiler handles distribution, coordination, storage and
testing.

```weft
add_todo = Command(|title: Text| -> Result(Id(Todo), [EmptyTitle]) {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  Ok(todos.insert(Todo { title: cleaned }))
})

todo_list = Query(|show: Filter| -> List(Todo) {
  todos.filter(|t| keeps(show, t)).sort_by(.created)
})
```

Start with [The big idea](https://pickleboyonline.github.io/weft/docs/) on
the docs site.

## What's here

| Path | What |
|---|---|
| [`website/docs/reference/`](website/docs/reference/) | The language reference. It is the specification. |
| [`docs/`](docs/) | Design notes: why the language looks as it does, and its open decisions. |
| [`examples/todo-lists`](examples/todo-lists/README.md) | A small CRUD app written in the designed syntax. |
| [`tree-sitter-weft`](tree-sitter-weft/README.md) | The grammar, with every ambiguity resolved explicitly. |
| [`crates/`](crates/) | The checker: `weft check`, `weft explain`, `weft fmt`, `weft doctest`. It parses and checks; it never generates code. |
| [`editors/vscode`](editors/vscode/README.md) | A VS Code extension that highlights `.weft` files. |
| [`website/`](website/README.md) | The documentation site (Docusaurus). |
| [`refs/`](refs/README.md) | Related projects to clone and read, and why each matters. |

## Working on it

`make` lists every task. The main ones:

```bash
make test
```

runs the checker's tests, checks the example project and every weft code
block in the docs, and runs the formatter and highlighting tests.

```bash
make site
```

runs the docs site at http://localhost:3000. Pushes to `main` publish it to
<https://pickleboyonline.github.io/weft/>.
