# Weft language design

**Status:** draft proposal, 2026-11-14. Nothing here is implemented. Section 14
lists the decisions that are still open.

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

1. **What**: the logical program. Data, commands that change it, queries over
   it, external effects and invariants, written as if everything ran on one
   machine.
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
- Code is pure by default. Effects are visible at the call site and precisely
  typed.
- Deterministic simulation is built in, not bolted on.
- People and AI agents can both read and write it. Compiler errors are exact fix
  instructions.

**Non-goals for v0**

- General protocol synthesis. Templates plus coordination analysis are the
  practical version.
- User-defined effects that capture continuations across machines (section 11).
- Replacing general-purpose languages. Externs (section 10) are the way out.

## 3. Principles

| # | Principle | From |
|---|---|---|
| 1 | Pure by default. Anything that changes the world or isn't deterministic ends in `!` and has a `=>` arrow; pure functions use `->`. Purity is inferred and checked. | Roc |
| 2 | Every function's type carries an inferred *effect row* saying exactly what it touches, e.g. `<write todos, reject [EmptyTitle]>`. | Koka |
| 3 | Platforms provide every effect; the language has no I/O of its own. The compiler picks each platform's implementations at compile time. | Roc |
| 4 | Only effects that resume once, or never, may cross machines. Continuation-capturing control stays on one machine. | Koka's operation kinds |
| 5 | Errors and messages are structural tags, so a server's rejection reasons flow into UI types and `match` is exhaustive. | Roc |
| 6 | Clients are components with local `state`, changed only by named `action`s; effects leave as commands. It compiles to the Elm Architecture: pure transitions, with messages as data. | Elm (the model); Svelte, Redux Toolkit (the syntax) |
| 7 | `{ ..r, f: v }` updates in place when nothing else references `r`. | Roc, Koka (Perceus) |

## 4. A complete example

The running example is a todo app: a server-side table, commands and queries, and
a browser client, all in one file.

```weft
module todos

# ── Data ──────────────────────────────────────────────────────

record Todo {
  key id:  Id   = new_id!()          # random and time are effects too;
  title:   Text                      # the sim platform makes them deterministic
  done:    Bool = False
  created: Time = now!()
}

Filter : [All, Active, Done]

table todos : Todo

# ── Logic: written as if on one machine ──────────────────────

command add_todo!(title: Text) {
  trimmed = title.trim()
  if trimmed.is_empty() { reject EmptyTitle }
  todos.insert!(Todo { title: trimmed })
}

command toggle!(id: Id) {
  todos.update!(id, |t| { ..t, done: not t.done })
}

command clear_done!() {
  todos.delete!(|t| t.done)
}

keeps = |show, t| match show {
  All    -> True
  Active -> not t.done
  Done   -> t.done
}

query todo_list(show: Filter) -> List(Todo) =
  todos.filter(|t| keeps(show, t)).sort_by(.created)

query remaining() -> U64 =
  todos.count(|t| not t.done)

# ── Client: local state, changed by actions ──────────────────

client App {
  state draft   = ""
  state show    = All
  state problem = None

  view = main {
    form(on_submit: add) {
      input(bind: draft, placeholder: "What needs doing?")
    }
    match problem {
      None       -> nothing
      EmptyTitle -> p(class: "error") { "Title can't be empty" }
    }
    ul {
      for t in todo_list(show) key t.id {
        li {
          checkbox(checked: t.done, on_change: tick(t.id))
          span(class: if t.done "done" else "") { t.title }
        }
      }
    }
    footer {
      span { "${remaining()} left" }
      for f in [All, Active, Done] {
        button(on_click: filter(f), pressed: show == f) { f.to_str() }
      }
      button(on_click: clear) { "Clear done" }
    }
  }

  action add {
    problem = None
    do add_todo!(draft) else |err| problem = err
    draft = ""
  }

  action tick(id: Id)     { do toggle!(id) }
  action filter(f: Filter) { show = f }
  action clear            { do clear_done!() }
}

check titles_not_blank = todos.all(|t| not t.title.trim().is_empty())
```

The file contains no fetch calls, API routes, loading code, cache
invalidation, or serialization. `App`'s `view` reads the query `todo_list(show)`
as if the data were local. Actions change local state directly and send commands with `do`.

## 5. Effects

### 5.1 Two levels

- **`!` with `=>`** marks a function that changes the world or isn't
  deterministic: writes, rejects, randomness, time, externs. A function is
  effectful if it calls an effectful function. Only effectful functions may
  call effectful functions.
- **An effect row with `->`** marks *reactive reads* of tables and queries.
  Reads are deterministic for a given snapshot of the data, so they stay
  unmarked at the call site. The compiler still sees them and turns them into
  subscriptions and sync.

### 5.2 Inferred types for the example

The compiler infers these; the IDE shows them on hover.

```weft
add_todo!   : Text => {}  ! <write todos, reject [EmptyTitle], random, time>
toggle!     : Id   => {}  ! <write todos>
clear_done! : {}   => {}  ! <write todos>
keeps       : Filter, Todo -> Bool
todo_list   : Filter -> List(Todo)  ! <read todos>
remaining   : {}     -> U64         ! <read todos>
App.problem : [None, EmptyTitle]   # inferred from `problem = err` in `add`
App.add     : action                ! <set draft, problem; do add_todo!>
App.tick    : Id -> action          ! <do toggle!>
App.view    : Html                  ! <read todo_list, remaining>
```

An action's row lists the state it sets and the commands it sends. Setting
local state isn't a change to the world, so actions don't carry `!`: they
compile to pure transitions (section 7).

### 5.3 What the compiler does with them

| Inferred | Consequence |
|---|---|
| `write todos` | The command runs where `todos` lives, plus an optimistic copy on clients that hold a local copy of it. |
| `reject [EmptyTitle]` | The check runs on both client and server, and the caller must handle the rejection (section 7). |
| `read todos` in queries that a client reads | Live subscriptions, or replication of `todos` to the client, whichever the cost model prefers. |
| An overwrite, as in `toggle!` | Not monotonic, so concurrent writes need a resolution rule. Insert-only logic is monotonic and needs no coordination (the CALM theorem). |
| `random`, `time` | Provided by each platform. The sim platform derives them from the run's seed. |

### 5.4 Rejections are tracked end to end

`reject` is an effect that never resumes: it aborts the transaction. Its
reasons are structural tags that become part of the command's type. If
`add_todo!` later gains `reject TooLong`, the inferred type of `App`'s `problem`
state grows to `[None, EmptyTitle, TooLong]`, and the compiler flags the `match`
on it in `App`'s view as no longer exhaustive. Server-side error cases are
checked all the way into the UI.

## 6. Declarations

| Declaration | May read tables | May write | May `reject` | May call externs | Runs where |
|---|---|---|---|---|---|
| `record`, tag type | | | | | data only |
| `table` | | | | | placed by the deployment |
| pure function (`f = \|x\| …`) | | | | | anywhere; inlined |
| `query` | ✓ | | | | where its data is (server, or a client's local copy) |
| `command` | ✓ | ✓ | ✓ | ✓ | where its data lives; also optimistically on clients |
| `client`, `component`, `page` (`state`, `view`) | through queries | | | | the client's machine |
| `action` | through queries | its own component's `state` | | only through `do` | the client's machine |
| `check` | ✓ | | | | evaluated by the simulator |
| `effect`, `handler` | per operation | | | | section 11 |
| `extern` | | | ✓ | is one | machines whose platform provides its capability |

Writing a table inside a `query` is a compile error, not a runtime surprise. So
is assigning to `state` anywhere but an action.

## 7. Clients

**Decided (2026-11-17):** clients are written as components with local state,
changed by named actions. This replaces the earlier Elm-style client (a
`Model`, a `Msg` union and an `update` function), which was correct but heavy:
every interaction needed a message variant plus an `update` branch, and all
state was global. The compiled form is still the Elm Architecture (7.5), so
nothing the compiler or simulator relies on changes.

### 7.1 The pieces

- **`client`** is the root of a UI, and what a deployment places
  (`place App on browser`). It's also the layout around every page.
- **`page`** is a component for one URL; its parameters come from the URL
  (section 9.4).
- **`component`** is a reusable piece of UI with parameters and its own state.
  Component names are capitalized, and arguments are named like markup
  attributes (`ItemRow(item: i)`), so components read differently from markup
  elements (`li`) and functions.
- **`state name = initial`** declares local state. Its type is inferred, or
  written out: `state selected: [None, Some(Id)] = None`.
- **`view = …`** is the markup: a pure function of the component's parameters,
  its state and the queries it reads. It can't assign.
- **`action name(params) { … }`** is the only place state changes.

A component, from the todo-lists example:

```weft
component ItemRow(item: Item) {
  state editing = False
  state draft   = ""
  state problem = None

  view = li {
    checkbox(checked: item.done, on_change: done(not item.done))
    if editing {
      form(on_submit: save) {
        input(bind: draft)
        button(on_click: cancel) { "Cancel" }
      }
    } else {
      span(class: if item.done "done" else "", on_click: edit) { item.title }
    }
    match problem {
      None       -> nothing
      EmptyTitle -> p(class: "error") { "An item needs a title" }
    }
    button(on_click: remove) { "Delete" }
  }

  action edit {
    editing = True
    draft = item.title
  }

  action cancel { editing = False }

  action save {
    editing = False
    do rename_item!(item.id, draft) else |err| problem = err
  }

  action done(d: Bool) { do set_done!(item.id, d) }
  action remove        { do delete_item!(item.id) }
}
```

### 7.2 Actions

- **Assignments set state.** Inside an action, `draft = ""` sets the
  component's state named `draft`. Any other name introduces a local value,
  as in commands (`cleaned = draft.trim()`). A local can't reuse a state's
  name.
- **Statements run in order,** and later ones see earlier assignments.
- **`do` sends a command.** It evaluates its arguments where it appears, and
  sends the command when the action finishes. That's also when the command is
  applied optimistically to the client's local copy of the data.
- **`else` is required exactly when the command can reject.** `do
  rename_item!(…)` without `else` doesn't compile; `do set_done!(…)` needs
  none. The body after `else |err|` is a small action. It runs when the
  rejection arrives, against the component's state at that time. If the
  component no longer exists, the handler is dropped; the optimistic change is
  still rolled back.
- **Events take actions.** `on_click: save` passes an action, and
  `on_change: done(True)` passes one with its arguments. If the event carries
  a value (text, a checked flag), it's passed as the action's last argument:
  `on_input: search` with `action search(q: Text)`.
- **`bind:` connects an input to state both ways.** `input(bind: draft)` needs
  no action for keystrokes.
- **Data goes down, actions go up.** A parent passes its own action to a child
  as a parameter: `Sidebar(selected: selected, on_select: select)`. The child
  declares it as `on_select: Id -> action` and uses it like any other action:
  `on_click: on_select(row.list.id)`.
- **`do` also runs another action.** Inside an action, `do on_deleted()` runs
  an action (often one passed down from a parent) right after this one
  finishes. So `do` always means "after this action, do this": send a command,
  or run an action.

Markup is typed against the actions in scope, so a misspelled action, or one
given the wrong arguments, is a compile error.

### 7.3 Identity and lifetime

A component's state lives as long as the component is rendered in the same
place. Inside a keyed loop (`for i in … key i.id { ItemRow(item: i) }`), the key
decides which row's state is which: deleting a row discards its state, and a
row that's re-added starts fresh.

### 7.4 Reading server data

A component's `view` calls the query `todo_list(show)` as if the data were
local. The compiler *elaborates* the client into a complete one. It adds hidden
state holding the client's local copy of synced data, a hidden action that
applies incoming changes, and subscriptions derived from which queries each
`view` reads with which arguments. After elaboration, every `view` is a pure function of state
again.

### 7.5 What it compiles to

Each component compiles to the Elm Architecture. For `App` in section 4, the
compiler generates the equivalent of:

```weft
# generated, not written
Msg : [Add, Tick(Id), Filter(Filter), Clear, DraftChanged(Text), AddRejected([EmptyTitle])]

update = |s, msg| match msg {
  Add             -> ({ ..s, problem: None, draft: "" }, do add_todo!(s.draft) else AddRejected)
  Tick(id)        -> (s, do toggle!(id))
  Filter(f)       -> ({ ..s, show: f }, none)
  Clear           -> (s, do clear_done!())
  DraftChanged(t) -> ({ ..s, draft: t }, none)
  AddRejected(e)  -> ({ ..s, problem: e }, none)
}
```

- **Each action becomes a message variant,** with its parameters as the
  payload. `bind:` adds a `…Changed` variant, and each `else` handler adds a
  `…Rejected` one.
- **Assignments become record updates.** That's local mutation that can't
  escape the action, which is how Koka keeps functions with local `var`s pure.
  Redux Toolkit does the same in JavaScript: you write `state.done = true` in a
  reducer, and it produces an immutable update.
- **Messages are addressed by their component's place in the tree,** e.g.
  `App / ItemRow(key: 42) / Save`. Every component's state is kept as one tree,
  so logging, replay and debugging see all of it.
- **The surface syntax doesn't fix the runtime strategy.** The browser platform
  could run this on a small Elm-Architecture runtime, or compile it to
  fine-grained signals, as Svelte compiles its components.

### 7.6 How the simulator drives a client

The simulator doesn't generate arbitrary messages. It interacts with what's
rendered: it picks among the actions attached to elements currently on screen,
and types random text (including empty text) into bound inputs. A message no
person could produce, such as `Save` for a row that isn't shown, never occurs.

## 8. Platforms and deployment

A **platform** bundles three things, as a Roc platform does:

- an implementation of every built-in effect (read, write, reject, random,
  time, dom, …);
- a list of capabilities it provides;
- a host language for implementing externs.

weft would ship standard platforms (`weft/local`, `weft/server-rust-postgres`,
`weft/browser-ts`, `weft/sim`), and anyone could write more, such as
`edge-cloudflare` or `ios-swift`.

Deployment lives in its own file and maps machines to platforms and data to
machines:

```weft
deploy local {
  machine laptop on "weft/local"                 # one process, in memory: the reference semantics
  place everything on laptop
}

deploy prod {
  machine server  on "weft/server-rust-postgres"
  machine browser on "weft/browser-ts" { count: many, offline: True }
  place todos on server
  place App   on browser
  # optional hint; otherwise the cost model decides:
  # sync todos to browser: replicate
}

simulate flaky_wifi {
  topology: prod                                 # prod's machines and placement...
  platform: "weft/sim"                           # ...on simulated platforms
  clients: 2
  network: drop 10%, latency 5ms..2s, offline 30%
  steps:   500 random actions per client
}
```

`deploy local` is the reference semantics from section 1: the same program on
one machine, with no distribution at all.

This example spells everything out. In a real project, conventions imply most
of it (section 9.3): `deploy local` is generated, tables go on the machine
named `server` and the client on the one named `browser`, and `simulate`
defaults to `topology: prod` and `platform: "weft/sim"`.

The same source compiles to different plans under different hints or data
sizes. For a personal todo list, replicating all of `todos` to the browser and
evaluating queries locally is cheapest: filters switch instantly and the app
works offline. For a million rows, running queries on the server and pushing only
results is cheapest.

## 9. Project layout and conventions

**Decided (2026-11-17):** a weft project follows a fixed layout and naming
conventions, in the spirit of Rails. The compiler sees the whole program, so
the layout can't change what a program means. What it does is decide where
everything goes, so projects are predictable for people, agents and
generators, and let deployment files state only what's actually a choice.

### 9.1 The layout

From [examples/todo-lists](../examples/todo-lists/README.md):

```text
todo-lists/
  weft.toml                  the project's name
  data/                      records and their tables, one record per file
    todo_list.weft
    item.weft
    filter.weft
  commands/                  every write, grouped by the table it's mostly about
    lists.weft
    items.weft
  queries/                   every read the client uses, grouped the same way
    lists.weft
    items.weft
  ui/
    App.weft                 the client: the layout around every page
    pages/                   one page per URL
      index.weft             /
      lists/[id].weft        /lists/:id
    components/              one component per file, named after it
      Sidebar.weft
      ListPanel.weft
      ItemRow.weft
  checks/                    invariants: the project's only tests
    lists.weft
    items.weft
  deploy/                    one file per deployment other than local
    prod.weft
  simulate/                  one file per simulation
    two_devices.weft
    flaky_wifi.weft
  schema/                    generated and checked in (9.5)
```

| Folder | Holds | Rails equivalent |
|---|---|---|
| `data/` | `record`s, their `table`s, and shared tag types | `app/models` |
| `commands/` | `command`s | controller actions, without HTTP |
| `queries/` | `query`s, and pure helpers they use | model scopes |
| `ui/` | the `client`, `page`s and `component`s | `app/views` plus `config/routes.rb` |
| `checks/`, `simulate/` | `check`s and simulations | `test/` |
| `deploy/` | deployments | `config/environments` |
| `schema/` | generated schema history | `db/schema.rb` |

### 9.2 Naming

- **No `module` lines.** A project is one module, named in `weft.toml`, and
  every top-level name is visible throughout it.
- **Files are named after what they hold.** `data/item.weft` holds
  `record Item`. `ui/components/ItemRow.weft` holds `component ItemRow`.
  Commands, queries and checks are grouped by table: `commands/items.weft`.
- **Records are singular** (`Item`), **tables plural** (`items`). Tables are
  always declared (`table items : Item`); weft never guesses an English plural.
- **Commands are verbs** (`add_item!`), **queries are nouns** (`items_in`),
  and **components and pages are capitalized** (`ItemRow`, `ListPage`).

### 9.3 Defaults deployments don't restate

- **`deploy local` is generated.** It's the reference, so it never varies and
  is never written.
- **Tables go on the machine named `server`, and the client on the machine
  named `browser`,** unless a `place` line says otherwise.
- **Simulations default to `topology: prod` and `platform: "weft/sim"`.**
- **`weft explain` shows every default it applied,** and why. A convention you
  can't see is the part of Rails people complain about.

With these, the example's whole production deployment is:

```weft
deploy prod {
  machine server  on "weft/server-rust-postgres"
  machine browser on "weft/browser-ts" { count: many, offline: True }

  consistency lists, items: session
}
```

### 9.4 Pages and routes

- **Each file under `ui/pages/` is a page,** and its path is its URL:
  `index.weft` is `/`, and `lists/[id].weft` is `/lists/:id`.
- **A page is a component whose parameters come from the URL:**
  `page ListPage(id: Id) { … }`. If a parameter doesn't parse, the client
  shows its not-found page.
- **`ui/App.weft` is the layout around every page.** `outlet` in its view marks
  where the current page goes.
- **Links are typed.** `link(to: ListPage(id: list.id)) { … }` links to a page
  by name and parameters, so a broken link is a compile error. A link to the
  current page is marked as current.
- **Actions navigate with `do go(Home)`.**
- **The URL is state like any other.** The simulator follows links on screen
  just as it clicks buttons.

### 9.5 Generated files

- **`schema/`** records each deployed version of every table. The compiler
  diffs against it to produce migrations, and to know what older offline
  clients expect. It's generated, but checked in and reviewed, like Rails'
  `db/schema.rb`.
- **Generators** write ordinary code that you then own, rather than adding
  invisible behavior. This is a planned command-line interface:

  ```bash
  weft new todo-lists
  weft generate resource Item title:Text done:Bool list:Id
  weft generate component ItemRow
  weft generate page lists/[id]
  weft generate simulation flaky_wifi
  ```

  `generate resource` would write the record and table, create, rename and
  delete commands, a list query, a row and a form component, and a
  not-blank check.

## 10. Escape hatches

An **extern** is code weft can't see into. You declare it in weft and implement
it in each platform's host language. Its capability and its simulation model
are both required:

```weft
extern send_email! : Text, Text => {} ! <reject [Bounced]>
  needs email
  sim { ok 95%, else Bounced, latency 50ms..2s }
```

Only machines whose platform provides `email` can run it.

Escape hatches exist at four levels, from least to most manual:

1. an extern, for code the compiler can't see into;
2. a sync or protocol hint, to pin a choice the compiler would otherwise make;
3. explicit placement of data and code;
4. writing the physical layer (locations, messages, handlers) directly.

## 11. User-defined effects

Users can declare their own effects and handlers, as in Koka:

```weft
effect Audit {
  fun note(event: Text)                 # `fun`: resumes exactly once
}

command clear_done!() {
  n = todos.delete!(|t| t.done)
  Audit.note!("cleared ${n}")
}

handler audit_table for Audit {
  fun note(event) { audit_log.insert!(Entry { event }) }
}
```

Koka distinguishes kinds of effect operations, and weft turns that into a
placement rule:

| Operation kind | Resumes | May be handled on another machine | Lowers to |
|---|---|---|---|
| `fun` | exactly once | yes | request/response |
| `final` (e.g. `reject`) | never | yes | abort plus error |
| `ctl` | any number of times (captures a continuation) | no, same machine only | local control flow |

A continuation should never travel over the network, so generators,
backtracking and async-style control stay local.

## 12. Compilation pipeline

The physical side of this pipeline is sketched in two concept notes:
[jax-analogy.md](jax-analogy.md) (what weft borrows from how JAX handles
hardware) and [infrastructure-primitives.md](infrastructure-primitives.md)
(storage, caching and compute as primitives instead of products).

```
front ends ─▶ logical IR ─▶ (+ placement, consistency, platforms) ─▶ physical IR ─▶ lowerings
                 │                                                        │
                 └────── reference semantics (oracle) ────── simulator ───┘
```

How each part of the example lowers under `deploy prod`:

| Source | Lowers to |
|---|---|
| Pure functions | Inlined into every target that needs them |
| Commands | A Rust transaction against Postgres on the server, plus an optimistic TypeScript copy against the browser's local data; the server confirms it, or the client rolls back and replays its pending changes |
| Queries | A SQL query plus a change feed, or local TypeScript evaluation if `todos` is replicated |
| `App` and its components | TypeScript: each component compiled to the Elm Architecture (section 7.5) on a small runtime, or to fine-grained signals |
| `table todos` | Schema, migrations, and indexes inferred from use (`sort_by(.created)`) |
| `!` effects | Calls into each platform; the sim platform runs them deterministically from a seed |

## 13. Testing

- **`check`** declarations are domain invariants, evaluated by the simulator.
- **Convergence is always checked.** Once sync settles, every client's query
  results must equal the server's. Nobody writes this check; the compiler derives it.
- **`simulate`** blocks drive many simulated clients through the actions on
  their screens (section 7.6), with network faults and offline periods.
  Because actions and views compile to pure functions and every effect is
  declared, a failing seed replays exactly.

Simulation finds counterexamples; it can't prove their absence.
[testing-and-proof.md](testing-and-proof.md) compares it with property-based
testing, model checking and proof (including Bend 2's laws). It proposes
letting the same `check` be simulated, exhausted on a small configuration, or
proven per command (open decision 13).

## 14. Front ends

The IR is the product, and the language is one front end for it. A Python
embedded DSL that emits the same IR remains useful: it proves the semantics
early, and AI models already know Python, whereas they have never seen weft. A
new language needs mitigations for that:

- a small grammar close to languages models know (Rust, Roc, Elm);
- compiler errors that read as exact fix instructions;
- a dense language reference that agents can load into context.

Tooling is most of the cost of a language: parser, typechecker, error
messages, formatter, language server and highlighting. A plausible stack is
tree-sitter for editor support plus a Rust compiler built on an incremental
query engine such as salsa.

## 15. Open decisions

1. **Reactive reads.** Unmarked, as proposed, or marked with `!` like every
   other effect, as Roc would do? *Recommendation:* unmarked, so queries stay
   readable.
2. **Effect rows.** Inferred everywhere, or required on public boundaries?
   *Recommendation:* required on commands and externs, since they form the
   API and a changed row is a changed contract; inferred elsewhere.
3. **Structural or nominal types.** Structural tags let errors flow into UI
   types for free; nominal types give better error messages.
   *Recommendation:* structural with optional names, as Roc does.
4. **User-defined handlers in v1.** *Recommendation:* built-in effects only in
   v1, then `fun` and `final` user effects, with general `ctl` much later.
5. **Loading and offline state in client views.** Show the last known value
   with explicit status available on request, or force views to handle a
   `Loading | Ready` type? *Recommendation:* last known value by default.
6. **Conflict semantics for optimistic commands.** Replay pending local
   changes on top of the server's (the Replicache/Zero approach), or per-field
   merge rules? *Recommendation:* replay by default, with merge rules as an
   opt-in per field.
7. **Users and authorization.** Not yet designed. Per-user todos
   (`owner = current_user!()`, placed per user) are the first test of whether
   placement stays a one-line concern.
8. **Rejections for a component that's gone.** When a rejection arrives after
   its component was removed, drop the `else` handler (as proposed), or pass
   the rejection to the nearest ancestor that declares a handler?
   *Recommendation:* drop it, with an optional client-wide handler for
   rejections nobody caught.
9. **Assignment syntax in actions.** Bare names (`draft = ""`, as proposed,
   where a state's name means assignment), or explicit
   (`state.draft = ""`)? *Recommendation:* bare names, with a compile error
   when a local would reuse a state's name.
10. **Folders as a project grows.** Keep grouping by layer (`commands/`,
    `queries/`, …), or switch to grouping by feature (`lists/`, `items/`, each
    with its own data, commands and UI), as Phoenix contexts and Django apps
    do? *Recommendation:* layers, with subfolders inside a layer once one gets
    crowded.
11. **Namespaces.** One flat namespace per project (as proposed), or folders
    as namespaces? *Recommendation:* flat until names actually collide.
12. **Routing beyond the basics:** nested layouts, query-string parameters,
    redirects, a not-found page per section. *Recommendation:* one layout
    (`App`) and path parameters only, until an example needs more.
13. **How strongly a `check` is verified.** Simulation only, or also
    exhaustive checking of small configurations, and proofs that each command
    preserves a check on one machine (with a person writing the check and an
    AI writing the proof, as in Bend 2)? See
    [testing-and-proof.md](testing-and-proof.md). *Recommendation:* simulation
    in v1, then bounded exhaustive checking, which reuses the simulator. Proofs
    come later, and never replace simulation: the compiler, platforms and
    externs are where verified systems have failed.

## 16. Prior art

Clones live in `refs/` (see [refs/README.md](../refs/README.md)).

| Project | What weft takes from it |
|---|---|
| Roc (`refs/roc`) | Platforms, `!`/`=>` purity inference, structural tag unions |
| Koka (`refs/koka`) | Effect rows, handlers, operation kinds, Perceus |
| Elm | The model clients compile to: pure transitions, messages as data |
| Svelte, Redux Toolkit, Hyperapp | The client syntax: local state, `bind:`, named actions with messages generated from them, mutation written in place and compiled to immutable updates |
| JAX (`refs/jax`) | Small IR, primitives with per-transformation rules, sharding annotations |
| Hydro (`refs/hydro`) | Typed locations, lattices, a compiler stack for distributed programs |
| Wing (`refs/wing`) | Abstract resources with one implementation per target, including a simulator |
| TigerBeetle, Maelstrom (`refs/tigerbeetle`, `refs/maelstrom`) | Deterministic simulation, checking against consistency models |
| Replicache / Zero | Optimistic commands that run on both client and server |
| Rails, Phoenix | A fixed project layout, generators that write ordinary code, a checked-in schema |
| SvelteKit, Next.js | Pages as files, with the path as the URL |
