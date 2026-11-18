# Weft language design

**Status:** draft design, 2026-11-18. Nothing is implemented.

The **language reference** on the documentation site (`website/docs/reference/`)
is the specification: every construct and rule. This document explains *why*
the language looks the way it does, which alternatives were considered, and
what is still undecided (section 8).

## 1. The problem

Weft is "JAX for general software engineering." You write an application's
logic once, in a restricted model with explicit escape hatches. A compiler then
takes over the work that usually gets hand-built around that logic:
distribution, coordination, storage, sync, and testing. The output should match
hand-built systems on correctness and come close on performance.

The first scaffold (commit `fb58534`) got the layer wrong. Users wrote the
distributed protocol themselves (messages, retries, heartbeats) and the
compiler only checked it. That is the *physical* layer: what the compiler should
produce, not what people should write.

So weft separates three things that today's software mixes together:

1. **What**: the logical program. Data, functions that change it, functions
   that read it, external effects and invariants, written as if everything ran
   on one machine.
2. **Where, and how strongly**: placement and consistency. Which data lives
   where, what may be stale, what must work offline. These play the role JAX's
   sharding annotations play.
3. **On what**: platforms. Each machine in a deployment runs on a platform that
   provides its effects and capabilities: a server, a browser, an edge node, a
   simulator.

The compiler maps (1) onto (3) under the constraints of (2).

The split also gives testing an oracle. **The logical program, run on one
machine, is its own specification.** The simulator checks that the distributed
version stays within the declared consistency of that reference run. You get
distribution-bug testing without writing any distribution tests; you only write
the invariants of your domain.

## 2. Goals and non-goals

**Goals**

- One program covers every machine: backend, frontend and storage are a single
  compilation graph.
- The compiler chooses protocols (replication, sync, routing, caching) from a
  library of templates, guided by placement annotations and a cost model.
- Code is pure by default, and what changes the world is visible where it
  happens.
- Deterministic simulation is built in, not bolted on.
- People and AI agents can both read and write it. Compiler errors are exact
  fix instructions.

**Non-goals**

- General protocol synthesis. Templates plus coordination analysis are the
  practical version.
- Replacing general-purpose languages. Externs are the way out.

## 3. Principles

| # | Principle | From |
|---|---|---|
| 1 | Pure by default. A function whose name ends in `!` may change the world; nothing else can. | Roc |
| 2 | Every function's type carries an inferred *effect row* saying what it touches. The compiler uses rows to place code, sync data and decide where coordination is needed. | Koka |
| 3 | Platforms provide every effect; the language has no I/O of its own. | Roc |
| 4 | Only effects that resume once, or never, may cross machines. | Koka's operation kinds |
| 5 | What a function *is* (command, query, check, handler) follows from its effects and its folder, not a keyword. | Koka, Rails |
| 6 | Configuration is values: tables, jobs, externs, deployments and simulations are ordinary records of standard-library types. | Encore, Wing, Elm |
| 7 | Two kinds of syntax, declarations and expressions. No phrase-like mini-languages. | |
| 8 | Components keep local state, changed only by their own handlers, and compile to the Elm Architecture. | Svelte (feel), Elm (discipline) |
| 9 | Methods belong to types and are found by static dispatch; generic code states what it needs with `where`. No traits. | Roc |
| 10 | Every project has the same layout, and conventions supply what deployments don't need to say. | Rails |

## 4. How the pieces fit

The todo-lists example (`examples/todo-lists`) shows each piece in place:

| Piece | Where it lives | Reference page |
|---|---|---|
| Records and tables | `data/` | Data and tables |
| Commands: functions that write | `commands/` | Commands and queries |
| Queries: functions that read | `queries/` | Commands and queries |
| Components, pages and styles | `ui/` | UI |
| Invariants | `checks/` | Checks and tests |
| Deployments and simulations | `deploy/`, `simulate/` | Deployment and simulation |
| Externs, jobs, endpoints | `externs/`, `jobs/`, `endpoints/` | Externs, jobs and endpoints |
| User-defined effects | `effects/` | Effects |

A command, as it appears in the example:

```weft
add_item! : Id(TodoList), Text => Id(Item) ! <reject [EmptyTitle, NoSuchList], ..>
add_item! = |list, title| {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  if not lists.has(list) { reject NoSuchList }
  items.insert!(Item { list, title: cleaned })
}
```

And the component that calls it:

```weft
component ListPanel(list: TodoList) {
  state draft   = ""
  state problem = None

  add = || {
    problem = None
    add_item!(list.id, draft) else |err| problem = err
    draft = ""
  }

  view = section {
    form(on_submit: add) { input(bind: draft, placeholder: "Add an item") }
    ul {
      for i in items_in(list.id, All) { ItemRow(item: i) }
    }
  }
}
```

## 5. Decisions, and why

### Effects are typed, and `!` marks the ones that matter

Every function's effects are inferred, and its name ends in `!` exactly when it
can change the world or isn't deterministic. Table *reads* are tracked but
unmarked: they're deterministic for a given set of rows, and marking them would
put `!` on every query, hiding the writes that matter. Rows are required only
in command and extern signatures, which form the API; elsewhere they're
inferred and shown on hover.

### Roles come from effects and folders, not keywords

An earlier design had a keyword for every concept (`command`, `query`,
`check`, `job`, `table`, `extern`, `client`, `page`, `action`, `deploy`,
`simulate`), over 40 reserved words in all. The effect system already knew most
of what those keywords announced: a function that writes `items` can only run
where `items` lives, as a transaction. So a function's role now follows from
its effects plus its folder, and folders also mark the API boundary (only
functions in `commands/` are callable from clients).

The trade-off is explicitness: a role isn't spelled out on the declaration's
line. It's recovered by `!`, by the folder, and by the editor and
`weft explain`, which show each function's role.

Alternatives considered: one keyword per kind (explicit, but the language grows
with the framework); decorators like `@command` (a second syntax, and known for
hiding behavior); wrapping everything in library constructors like
`Command(…)` (used for configuration, below, but noisy for logic).

### Configuration is values

Tables, externs, jobs, endpoints, deployments and simulations are written as
`name = Type { … }`. They're type-checked and autocompleted like any other
value, and a package or platform can add new kinds without changing the
grammar. They must be top-level bindings, so the compiler can read them
without running anything, as Encore requires of its infrastructure
declarations.

### One syntax, no mini-languages

Earlier deployment and simulation syntax read like English
(`steps: 500 random actions per client`, `place lists on server`): every
construct invented its own phrase grammar. Now there are declarations and
expressions only, and configuration is named arguments or record fields with
ordinary values. Units (`5ms`, `64MB`, `10%`) and ranges (`5ms..=2s`) are
literals of real types, so they're usable anywhere.

### Components: Svelte's feel, Elm's discipline

The first client design was the Elm Architecture: a `Model`, a `Msg` union and
an `update` function. It was correct but heavy, since every interaction needed
a message variant plus a branch, and all state was global. A second design
added named `action`s. The final design follows Svelte:

- local `state`, assigned with plain `=`;
- handlers are ordinary functions in the component, named or inline;
- values computed from state are always up to date, with nothing to declare
  (weft is pure, so every such binding is already derived);
- `bind:` for inputs, scoped `style` blocks, `class` toggles.

What weft keeps from Elm is the discipline: state changes only in the
component's own handlers, never in `view` and never from outside, and state
is plain data. Because the language has no references and no shared mutable
objects, the compiler can find every assignment exactly, so it generates the
messages and update function Elm makes you write. That's what lets the
simulator replay any run.

Svelte's `$`-prefixed runes exist because Svelte lives inside JavaScript and
needed syntax that stays valid JavaScript. weft is its own language and
doesn't. Roc's convention of `$` on every mutable name was considered and left
out: Svelte's experience (moving from `$store` at every use to `$state` at the
declaration) favors clean reading, and `state` and `var` already mark the
declaration.

### Methods and `where` clauses instead of traits

Roc's model: methods belong to a type, `x.f()` finds `f` on `x`'s type at
compile time, and generic functions list the methods they need in a `where`
clause. There's nothing to declare or implement, operators are methods
(`==` is `equals`), and equality, ordering, text and hashing are derived for
all data. Rust-style traits or Haskell type classes were the alternative:
explicit, but a whole extra concept.

### Loops and local variables

Roc shows a pure language can have `for`, `while`, `break`, `continue` and
`return`, with `var` for variables that never escape their function. A
function with a `var` stays pure. This replaced an earlier rule that logic
could only use collection methods.

### Conventions over configuration

A fixed layout (section 4) lets deployments say only what's a real choice:
`local` is generated, tables go on `server`, the client on `browser`. Every
default is visible through `weft explain`, because invisible conventions are
the part of Rails people complain about. Pages are files whose path is their
URL. Routes are typed values, so a broken link is a compile error.

### Replay by default, merge by choice

Offline commands are replayed against the server's data in order, so every
command runs exactly as written on one machine's state. That's the
Replicache/Zero approach, and it keeps the one-machine meaning. A table field
can opt into a merge rule (`Latest`, `Union`, `Sum`) when convergence matters
more than replay. The compiler flags checks that depend on merged fields.

### Checks at three strengths

Checks are verified by random simulation, or by exhausting every order of a
few actions for a few clients. Exhausting finds most concurrency bugs, which
need only two or three actions to show up. See
[testing-and-proof.md](testing-and-proof.md).

## 6. Compilation pipeline

The physical side is sketched in two concept notes:
[jax-analogy.md](jax-analogy.md) (what weft borrows from how JAX handles
hardware) and [infrastructure-primitives.md](infrastructure-primitives.md)
(storage, caching and compute as primitives instead of products).

```
front ends ─▶ logical IR ─▶ (+ placement, consistency, platforms) ─▶ physical IR ─▶ lowerings
                 │                                                        │
                 └────── reference semantics (oracle) ────── simulator ───┘
```

How the todo-lists example lowers under its `prod` deployment:

| Source | Lowers to |
|---|---|
| Pure functions | Inlined into every target that needs them |
| Commands | A Rust transaction against Postgres on the server, plus an optimistic TypeScript copy against the browser's local data; the server confirms it, or the client rolls back and replays its pending changes |
| Queries | A SQL query plus a change feed, or local TypeScript evaluation over a replicated table |
| Components | TypeScript: each compiled to the Elm Architecture on a small runtime, or to fine-grained signals |
| Tables | Schema, migrations from `schema/`, and indexes inferred from use (`sort_by(.created)`) |
| `!` effects | Calls into each platform; the sim platform runs them deterministically from a seed |

## 7. Front ends

The intermediate representation is the product, and the language is one front
end for it. A Python embedded DSL that emits the same IR remains useful: AI
models already know Python, whereas they have never seen weft. A new language
needs mitigations for that:

- a small grammar close to languages models know (Rust, Roc, Svelte);
- compiler errors that read as exact fix instructions;
- a dense language reference that agents can load into context.

Tooling is most of the cost of a language: parser, type checker, error
messages, formatter, language server and highlighting. A plausible stack is
tree-sitter for editor support plus a Rust compiler built on an incremental
query engine such as salsa.

## 8. Open decisions

These belong to the project owner.

1. **Users and authorization.** Who can call which command, and see which
   rows. Per-user todo lists are the first test: whether ownership can be
   declared on a table (`Table(Item, owner: …)`) so placement and sync stay
   one-line concerns.
2. **How a check is proven.** Simulation and exhaustive checking are designed.
   Proving that every command preserves a check on one machine (a person
   writes the check, an AI writes the proof, as in Bend 2) needs a proof
   syntax. See [testing-and-proof.md](testing-and-proof.md).

Decided since earlier drafts, with the reasons above: reactive reads stay
unmarked; effect rows are required on commands and externs only; records are
nominal and tag unions structural; user-defined effects and handlers are part
of the language, with the cross-machine rule; loading states use `status(…)`;
optimistic commands replay by default with opt-in merge rules; a rejection
arriving after its component is gone is dropped; state is assigned with plain
names; projects group by layer in one namespace; routing has typed routes,
query parameters, layouts and a not-found page.

## 9. Prior art

Clones live in `refs/` (see [refs/README.md](../refs/README.md)).

| Project | What weft takes from it |
|---|---|
| Roc (`refs/roc`) | Platforms, `!` purity, structural tag unions, static dispatch, `where` clauses, loops with `var` |
| Koka (`refs/koka`) | Effect rows, handlers, operation kinds |
| Elm | The model components compile to: pure transitions, messages as data |
| Svelte | The component feel: local state, plain assignment, `bind:`, scoped styles |
| JAX (`refs/jax`) | Small IR, primitives with per-transformation rules, sharding annotations |
| Hydro (`refs/hydro`) | Typed locations, lattices, a compiler stack for distributed programs |
| Wing, Encore (`refs/wing`, `refs/encore`) | Infrastructure declared as values, read statically by the compiler |
| TigerBeetle, Maelstrom (`refs/tigerbeetle`, `refs/maelstrom`) | Deterministic simulation, checking against consistency models |
| Replicache / Zero | Optimistic commands that run on both client and server |
| Rails, Phoenix | A fixed project layout, generators that write ordinary code, a checked-in schema |
| SvelteKit, Next.js | Pages as files, with the path as the URL |
