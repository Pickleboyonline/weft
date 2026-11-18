---
sidebar_label: Status
description: What exists today, what doesn't, and which parts of these docs are still to come.
---

# Status

weft is a personal project: a tool for building end-to-end systems that are
robust, correct and fast without constantly re-engineering and re-checking
them. It's at the design stage.

## What exists

| Piece | Where |
|---|---|
| The language reference: every construct and rule | [Reference](/docs/reference/overview) |
| The design rationale, with its open decisions | [Design notes: Language design](/design/language) |
| Concept notes and research | [Design notes](/design/language): the JAX analogy, infrastructure primitives, testing and proof, prior art, a reading list |
| An example app written in the designed syntax | `examples/todo-lists` in the repo, with the gaps it exposed |
| A parser and checker for the design: the tree-sitter grammar, plus name resolution, effect and type inference, and the language's rules, with `weft explain <code>` for every error, and `weft fmt`. It reports problems and formats code; it compiles nothing. | `tree-sitter-weft/` and `crates/` in the repo (`make check`, `make doctest`) |
| A VS Code extension: highlighting and a file icon | `editors/vscode` in the repo (`make vscode` installs it) |
| This site | `website/` in the repo (`make site` runs it) |

## What doesn't exist yet

- The compiler: lowering, code generation and the runtime. The checker stops
  at diagnostics.
- The simulator.
- Any platform: local, server, browser or sim.
- Parts of the design itself. Distribution (copies of data on clients,
  offline use, consistency levels), authorization, and the client cache
  beyond refresh by table are open decisions: see
  [Language design](/design/language).

An early scaffold (a Python front end, an intermediate representation and a
deterministic simulator) was built and then set aside when the design moved up
a level. It's kept in the repo's history.

## Documentation still to come

The structure follows the [documentation plan](/design/docs-plan). These
sections will be written as the language takes shape:

| Section | What it will hold |
|---|---|
| **Tutorial** | One CRUD app grown step by step: the pure core, the client, effects, talking to the server (queries and commands), storage, deployment, the simulator |
| **How-to guides** | Adding a field, writing an effect, using an escape hatch, debugging a failing seed, deploying |
| **Concepts** | About effects · About remote functions and their guarantees · About the simulator |
| **Coming from…** | weft for Elm users, for Svelte users, for TypeScript + Postgres users, for Hydro users |
| **The CLI** | `weft new`, `generate`, `test`, `simulate`, `explain`, `deploy`, and their options |
