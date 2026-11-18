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

**Where the design stands.** Layer (2) is deliberately not designed yet.
Placement, replication, client copies of data, offline use and consistency
levels have consequences that are hard to prove and understand up front, and
committing to them early would turn weft into a DSL with an escape hatch for
every case the model didn't foresee. So the language commits to (1) and (3)
with one fixed shape: a server with a database, and clients. The client and
server talk through explicit, typed remote functions (`Query` and `Command`,
like SvelteKit's remote functions or Next.js server actions), and the language
adds guarantees that remove whole classes of bugs those apps have: typed and
validated calls, typed failures, writes that refresh the reads they affect,
retries applied once, atomic commands, N+1-free queries, and server-only code
that can't reach the client. Layer (2) is section 8's largest open question.

The split also gives testing an oracle. **The logical program, run on one
machine, is its own specification.** The simulator checks that the distributed
version stays within the declared consistency of that reference run. You get
distribution-bug testing without writing any distribution tests; you only write
the invariants of your domain.

## 2. Goals and non-goals

**Goals**

- One program covers every machine: backend, frontend and storage are a single
  compilation graph.
- Client and server talk through explicit remote functions, with guarantees
  that remove classes of bugs rather than features that hide them.
- The language stays general enough that ordinary needs (other services,
  sockets, timers, widgets) are written in it, not escaped from it.
- Code is pure by default, and the compiler knows every effect, reads
  included, without the developer marking any.
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
| 1 | Effects are inferred, never written. The compiler knows exactly what each function reads, writes and sends, and uses that for its guarantees: which queries a command refreshes, which code must stay on the server, what must not interleave. Names and types don't mark it. | Koka |
| 2 | Table reads are language-integrated queries with a guarantee: every read compiles to one database query, or one per level of nesting, or it's a compile error. | Links; Cheney, Lindley and Wadler |
| 3 | Platforms provide every effect; the language has no I/O of its own. | Roc |
| 4 | Only effects that resume once, or never, may cross machines. | Koka's operation kinds |
| 5 | Remote functions are explicit values, `Query(…)` and `Command(…)`: the client–server boundary is the app's API, so it's written down. Other roles (handlers, where a helper runs) follow from the code. No keywords; folders are a convention. | SvelteKit, Rails |
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
add_item : Id(TodoList), Text -> Result(Id(Item), [EmptyTitle, NoSuchList])
add_item = |list, title| {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  if not lists.has(list) { return Err(NoSuchList) }
  Ok(items.insert(Item { list, title: cleaned }))
}
```

And the component that calls it:

```weft
component ListPanel(list: TodoList) {
  state draft   = ""
  state problem = None

  add = || {
    problem = None
    let title = draft
    draft = ""
    add_item(list.id, title).await.on_err(|err| problem = err)
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

### Effects are inferred, as in Koka

The compiler infers every function's effects (reading or writing a table,
assigning component state, sending a command, navigating, time, randomness,
externs, user-defined effects) and nothing in the code marks them.

Two marked designs came first. The first combined Koka's effect rows with
Roc's `!` but exempted reads, state and sends from `!`, so `!` meant "some
effects, but not others". The second applied Roc's rule fully, with `!` on
every effect, but then components, inline handlers and record defaults were
still exceptions, and every query carried a `!`.

The deciding argument is weft's own premise. You write as if on one machine,
and the compiler handles distribution, so effects aren't hazards the
developer manages: the compiler and the simulator are what need them, and
they get them by inference. Marking existed only for human readers, and a
marker that's right most of the time is worse than none. What readers lose is
supplied by tooling: hover, highlighting of calls that read or write, and
`weft explain effects` as text for agents. Where an effect matters (in
`expect`, top-level constants, views, checks and queries on tables) the
compiler reports it by name.

Koka also gives effect polymorphism: `map` has its callback's effects, so
there's one `map`, not a pure one and an effectful one.

From Roc, weft takes errors as values: a command that can fail returns a
`Result`, and returning `Err` rolls it back. That replaced `reject`, an
effect-based mechanism.

Reads stay deterministic where it matters through snapshots: a command reads
one snapshot plus its own writes, a component reads one snapshot per render,
and the simulator chooses when snapshots advance.

User-defined effects and handlers come from Koka too, with operation kinds
deciding which handlers may run on another machine.

### Queries are language-integrated, with a guarantee

LINQ and ORMs translate ordinary code into SQL where they can and fall back,
or fail at runtime, where they can't, and nested results become N+1 queries.
Entity Framework Core 3.0 stopped evaluating untranslatable parts on the
client, which it had done with only a warning, because the fallback caused
performance problems nobody saw in testing.

The research answer (Cooper's work on Links; Cheney, Lindley and Wadler's
*A Practical Theory of Language-Integrated Query* and *Query Shredding*) is a
*query fragment* with a normalization theorem: code in the fragment always
becomes one query, or one per level of nesting in the result. weft adopts it:

- functions given to table methods must be in the fragment, and anything else
  is a compile error that says how to fix it, never a fallback;
- `rows()` is the one explicit way to fetch a whole table into memory;
- queries run on the server, and refresh when a command writes a table they
  read. Keeping results up to date incrementally (DBSP shows this works for
  any relational query) belongs to the client cache and sync design, still
  open.

Mainstream languages couldn't adopt this because mutation and references make
the fragment hard to enforce. weft is pure and has no references, so it can.

This is kept deliberately contained: it translates one function's table
reads into SQL for the one database, and makes no decision about where data
lives. What stays hard: matching weft's meaning exactly in SQL (text
comparison, overflow, empty aggregates), and knowing table sizes for index
choices.

### Remote functions are explicit; other roles aren't keywords

An earlier design had a keyword for every concept (`command`, `query`,
`check`, `job`, `table`, `extern`, `client`, `page`, `action`, `deploy`,
`simulate`), over 40 reserved words in all. Those became standard-library
values instead (`Table(…)`, `Job { … }`, `Deployment { … }`).

Two later drafts went further and inferred roles: first from effects plus
folders, then from effects plus use (a function a handler calls that writes
is a command). That was dropped for commands and queries. They're the
boundary between client and server: the app's API, where security,
validation and compatibility live, and the one place a reader most needs to
see. Inferring it also leaned on the distributed design, which isn't settled.
So they're explicit values, `Query(…)` and `Command(…)`, as in SvelteKit's
remote functions. Checks are `Check(…)` for a similar reason: whether a `Bool`
is an invariant is intent, not behavior.

Everything else stays inferred where inference is ordinary and local: a
handler is a component function that assigns state, and where a helper runs
follows from what it references. Folders are a convention.

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
- values computed from state and queries are always up to date, with nothing
  to declare;
- `bind:` for inputs, scoped `style` blocks, `class` toggles.

What weft keeps from Elm is the discipline: state changes only in the
component's own handlers and tasks, never in `view` and never from outside,
and state is plain data. Because the language has no references and no shared mutable
objects, the compiler can find every assignment exactly, so it generates the
messages and update function Elm makes you write. That's what lets the
simulator replay any run.

Svelte's `$`-prefixed runes exist because Svelte lives inside JavaScript and
needed syntax that stays valid JavaScript. weft is its own language and
doesn't. Roc's convention of `$` on every mutable name was considered and left
out: Svelte's experience (moving from `$store` at every use to `$state` at the
declaration) favors clean reading, and `state` and `let mut` already mark the
declaration.

### General client constructs, not special cases

The keyword problem had a second form. Each thing table queries didn't cover
(data from another service, a live socket, a canvas widget) got its own
compiler feature or a host-code escape hatch. React and Svelte need neither:
they provide a few general constructs (state, effects with cleanup, element
refs, async, stores, reusable stateful logic), and TanStack Query is ordinary
library code built from them. weft takes the same approach:

- **`.await`** waits for something that takes time: a command's result, an
  HTTP call, a timer, a socket. It replaced `then`/`else` on commands, and the
  rule that a failing command needs an `else` became a general one: a
  `Result` can't be ignored. It's postfix, as in Rust, so it chains
  (`rename_item(id, t).await.on_err(…)`).
- **`task`** is work tied to a component's life, after SwiftUI's `.task`: it
  starts when the component appears, is cancelled when it goes away, and
  restarts when its `key:` changes. Restarts are explicit because restarting
  on everything a task reads loops when the task assigns state it also reads.
  `defer` replaces cleanup functions.
- **Element handles** (`ref:`) are opaque data, like ids, with platform
  operations.
- **Stateful functions** declare `state` or `task` and play the role of custom
  hooks. React's rule of hooks is checked statically.
- **`Device` and `Memory` tables** are stores and caches, with the same
  queries and snapshots as shared data.
- **Platform modules** (`http`, `ws`, `dom`, `browser`, `secrets`) reach the
  outside world, so externs are left for host libraries only. Simulations
  model other services per host, and the one-machine comparison takes their
  responses as given.
- On the server, **`Job { watch: query }`** plus a table of pending work is
  the outbox pattern, written in weft, which is how a change triggers a call
  to another service when commands can't make one.

`.await` is the one thing marked in code even though effects aren't. It isn't
an effect: it marks where other handlers may run in between, so state read
before it may have changed after it. That changes what the code means, the
way `return` does, so it's visible.

### Methods and `where` clauses instead of traits

Roc's model: methods belong to a type, `x.f()` finds `f` on `x`'s type at
compile time, and generic functions list the methods they need in a `where`
clause. There's nothing to declare or implement, operators are methods
(`==` is `equals`), and equality, ordering, text and hashing are derived for
all data. Rust-style traits or Haskell type classes were the alternative:
explicit, but a whole extra concept.

### Loops and local variables

Roc shows a pure language can have `for`, `while`, `break`, `continue` and
`return`, with variables that never escape their function. A function with a
`let mut` stays pure. This replaced an earlier rule that logic could only use
collection methods.

### Borrowed from Rust

Beyond `let`, `let mut` and shadowing (below), weft takes the parts of Rust's
surface that suit it:

- `let … else` for a pattern that can fail, which is what commands do most
  (`let Some(item) = items.get(id) else { return Err(NoSuchItem) }`);
- `if let` for acting on one case, mostly in markup;
- or-patterns (`Active | Done -> …`), in `match` arms only, since `|` also
  delimits a lambda's parameters;
- rustc-style diagnostics: a stable code, the source line, labels on related
  places ("bound here"), notes, and `weft explain <code>` for a longer
  explanation with an example and the fix;
- one formatter with no options, `weft fmt`, as `rustfmt` and `gofmt` do it.

Rust's other surface doesn't fit: `fn` would be a second way to write what a
typed lambda already says; `List<Item>` is ambiguous with `<` (the reason
Rust needs `::<>`); `impl` and traits are covered by record methods and `where`
clauses; and weft has no semicolons, macros, lifetimes or references.

### `let`, `let mut`, and shadowing

Local bindings start with `let`, variables with `let mut`, as in Rust. Plain
`name = value` inside a block is always an assignment, to a `let mut` or a
`state`; at the top level and for record and component members it's a
definition.

An earlier design used `name = value` for both new bindings and assignments,
with `var` for variables, and banned shadowing. Without a keyword, a rebinding
(`cleaned = cleaned.lower()`) reads exactly like a mutation, so shadowing had
to go. But the ban made every top-level name off limits as a parameter (`|items|`
was an error in any project with an `items` table), and the checker had to
look names up to tell a binding from an assignment. With `let`, each line says
which it is, so shadowing is allowed, as in Rust; a `let` also lets the parser
read a destructuring pattern directly. Roc itself only warns on shadowing.

### Conventions over configuration

A fixed layout (section 4) lets deployments say only what's a real choice:
`local` is generated, tables go on `server`, the client on `browser`. Every
default is visible through `weft explain`, because invisible conventions are
the part of Rails people complain about. Pages are files whose path is their
URL. Routes are typed values, so a broken link is a compile error.

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
| Commands | An endpoint plus a typed client call, with argument validation and a request id; on the server, an atomic run (a Postgres transaction, or a single statement where that's provably equivalent) |
| Queries | An endpoint plus a typed client call, cached on the client by arguments and refreshed when a command writes a table they read; on the server, one SQL query per level of nesting |
| Components | TypeScript: each compiled to the Elm Architecture on a small runtime, or to fine-grained signals |
| Tables | Schema, migrations from `schema/`, and indexes inferred from use (`sort_by(.created)`) |
| Effects | Calls into each platform; the sim platform runs them deterministically from a seed |

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

1. **Users and authorization.** Who can call which query or command, and see
   which rows. One candidate is requiring an access rule on every remote
   function, so a public endpoint can't happen by accident. Per-user todo
   lists are the first test, including whether ownership can be declared on
   a table (`Table(Item, owner: …)`).
2. **How a check is proven.** Simulation and exhaustive checking are designed.
   Proving that every command preserves a check on one machine (a person
   writes the check, an AI writes the proof, as in Bend 2) needs a proof
   syntax. See [testing-and-proof.md](testing-and-proof.md).
3. **The client cache.** Query results are cached by arguments and refreshed
   by table. Still open: keeping results across pages, prefetching, how long
   unused results live, and optimistic updates (showing a command's effect
   before the server confirms), which today are absent rather than automatic.
4. **Distribution: placement, replication, offline and consistency.** Where
   data lives beyond one server and its database; copies of data on clients;
   offline writes and how they're replayed or merged; consistency levels;
   and plans chosen by a cost model. Earlier drafts proposed answers
   (deployment `placement` and `consistency` fields, replay with opt-in merge
   rules, the oracle checking declared consistency); they're recorded in
   [infrastructure-primitives.md](infrastructure-primitives.md) and the git
   history, and are deliberately not part of the language until their
   consequences are worked out.

Decided since earlier drafts, with the reasons above: effects are inferred
and never written, as in Koka, with no `!` marker; table reads are
language-integrated queries with a guarantee, contained to one database;
commands and queries are explicit `Query` and `Command` remote functions,
checks are `Check` values, and folders are a convention; commands are atomic
and fail by returning `Err`, which rolls them back; records are
nominal and tag unions structural; user-defined effects and handlers are part
of the language, with the cross-machine rule; clients use general constructs
(`.await`, `task`, `defer`, element handles, stateful functions, `Device` and
`Memory` tables, platform modules) instead of special cases, and externs are
for host libraries only; loading states use `status(…)`;
a failure
arriving after its component is gone is dropped; local bindings use `let` and
`let mut`, and shadowing is allowed; state is assigned with plain
names; projects group by layer, by convention, in one namespace; routing has typed routes,
query parameters, layouts and a not-found page.

## 9. Prior art

Clones live in `refs/` (see [refs/README.md](../refs/README.md)).

| Project | What weft takes from it |
|---|---|
| Roc (`refs/roc`) | Platforms, errors as `Result` values, structural tag unions, static dispatch, `where` clauses, loops with local variables |
| Koka (`refs/koka`) | Inferred effects, effect polymorphism, user-defined effects, handlers, operation kinds |
| Links, T-LINQ | Language-integrated query with a guarantee: the query fragment and normalization |
| Elm | The model components compile to: pure transitions, messages as data |
| Svelte | The component feel: local state, plain assignment, `bind:`, scoped styles |
| JAX (`refs/jax`) | Small IR, primitives with per-transformation rules, sharding annotations |
| Hydro (`refs/hydro`) | Typed locations, lattices, a compiler stack for distributed programs |
| Wing, Encore (`refs/wing`, `refs/encore`) | Infrastructure declared as values, read statically by the compiler |
| TigerBeetle, Maelstrom (`refs/tigerbeetle`, `refs/maelstrom`) | Deterministic simulation, checking against consistency models |
| Replicache / Zero | Optimistic commands that run on both client and server |
| Rails, Phoenix | A fixed project layout, generators that write ordinary code, a checked-in schema |
| SvelteKit, Next.js | Pages as files, with the path as the URL |
