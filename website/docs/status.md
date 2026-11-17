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
| The language proposal, with its open decisions | [Design notes: Language proposal](/design/language) |
| Concept notes and research | [Design notes](/design/language): the JAX analogy, infrastructure primitives, prior art, a reading list |
| An example app written against the proposal | `examples/todo-lists` in the repo, with the gaps it exposed |
| A VS Code extension: highlighting and a file icon | `editors/vscode` in the repo (`make vscode` installs it) |
| This site | `website/` in the repo (`make site` runs it) |

## What doesn't exist yet

- The compiler.
- The simulator.
- Any platform: local, server, browser or sim.

An early scaffold (a Python front end, an intermediate representation and a
deterministic simulator) was built and then set aside when the design moved up
a level. It's kept in the repo's history.

## Documentation still to come

The structure follows the [documentation plan](/design/docs-plan). These
sections will be written as the language takes shape:

| Section | What it will hold |
|---|---|
| **Tutorial** | One CRUD app grown step by step: the pure core, the client, effects, the server, storage, deployment, consistency, the simulator |
| **How-to guides** | Adding a field, writing an effect, using an escape hatch, debugging a failing seed, deploying |
| **Concepts** | About effects · About placement and consistency · About the simulator |
| **Reference** | The language, deployment options, the CLI and standard library, with rule IDs and tested examples |
| **Coming from…** | weft for Elm users, for TypeScript + Postgres users, for Hydro users |
