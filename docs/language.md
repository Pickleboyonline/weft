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

1. **What**: the logical program. Data, operations, derived views, external
   effects and invariants, written as if everything ran on one machine.
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
- User-defined effects that capture continuations across machines (section 10).
- Replacing general-purpose languages. Externs (section 9) are the way out.

## 3. Principles

| # | Principle | From |
|---|---|---|
| 1 | Pure by default. Anything that changes the world or isn't deterministic ends in `!` and has a `=>` arrow; pure functions use `->`. Purity is inferred and checked. | Roc |
| 2 | Every function's type carries an inferred *effect row* saying exactly what it touches, e.g. `<write todos, reject [EmptyTitle]>`. | Koka |
| 3 | Platforms provide every effect; the language has no I/O of its own. The compiler picks each platform's implementations at compile time. | Roc |
| 4 | Only effects that resume once, or never, may cross machines. Continuation-capturing control stays on one machine. | Koka's operation kinds |
| 5 | Errors and messages are structural tags, so a server's rejection reasons flow into UI types and `match` is exhaustive. | Roc |
| 6 | Clients follow the Elm Architecture: pure `update` and `view`, effects returned as data. | Elm |
| 7 | `{ ..r, f: v }` updates in place when nothing else references `r`. | Roc, Koka (Perceus) |

## 4. A complete example

The running example is a todo app: a server-side table, commands and views, and
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

view todo_list(show: Filter) -> List(Todo) =
  todos.filter(|t| keeps(show, t)).sort_by(.created)

view remaining() -> U64 =
  todos.count(|t| not t.done)

# ── Client: the Elm Architecture ──────────────────────────────

client App {
  Model : { draft: Text, show: Filter, problem: [None, EmptyTitle] }
  init  = { draft: "", show: All, problem: None }

  Msg : [Typed(Text), Submit, Toggle(Id), Show(Filter), ClearDone, Rejected(_)]

  update = |m, msg| match msg {
    Typed(text)   -> ({ ..m, draft: text, problem: None }, none)
    Submit        -> ({ ..m, draft: "" }, do add_todo!(m.draft) else Rejected)
    Toggle(id)    -> (m, do toggle!(id))
    Show(f)       -> ({ ..m, show: f }, none)
    ClearDone     -> (m, do clear_done!())
    Rejected(err) -> ({ ..m, problem: err }, none)
  }

  view = |m| main {
    form(on_submit: Submit) {
      input(value: m.draft, placeholder: "What needs doing?", on_input: Typed)
    }
    match m.problem {
      None       -> nothing
      EmptyTitle -> p(class: "error") { "Title can't be empty" }
    }
    ul {
      for t in todo_list(m.show) key t.id {
        li {
          checkbox(checked: t.done, on_change: Toggle(t.id))
          span(class: if t.done "done" else "") { t.title }
        }
      }
    }
    footer {
      span { "${remaining()} left" }
      for f in [All, Active, Done] {
        button(on_click: Show(f), pressed: m.show == f) { f.to_str() }
      }
      button(on_click: ClearDone) { "Clear done" }
    }
  }
}

check titles_not_blank = todos.all(|t| not t.title.trim().is_empty())
```

The file contains no fetch calls, API routes, loading code, cache
invalidation, or serialization. `view` reads `todo_list(m.show)` as if the data
were local, and `update` returns commands as data.

## 5. Effects

### 5.1 Two levels

- **`!` with `=>`** marks a function that changes the world or isn't
  deterministic: writes, rejects, randomness, time, externs. A function is
  effectful if it calls an effectful function. Only effectful functions may
  call effectful functions.
- **An effect row with `->`** marks *reactive reads* of tables and views.
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
App.update  : Model, Msg -> (Model, Cmd(Msg))
App.view    : Model -> Html(Msg)    ! <read todo_list, remaining>
```

### 5.3 What the compiler does with them

| Inferred | Consequence |
|---|---|
| `write todos` | The command runs where `todos` lives, plus an optimistic copy on clients that hold a local copy of it. |
| `reject [EmptyTitle]` | The check runs on both client and server, and the caller must handle the rejection (section 7). |
| `read todos` in views that a client reads | Live subscriptions, or replication of `todos` to the client, whichever the cost model prefers. |
| An overwrite, as in `toggle!` | Not monotonic, so concurrent writes need a resolution rule. Insert-only logic is monotonic and needs no coordination (the CALM theorem). |
| `random`, `time` | Provided by each platform. The sim platform derives them from the run's seed. |

### 5.4 Rejections are tracked end to end

`reject` is an effect that never resumes: it aborts the transaction. Its
reasons are structural tags that become part of the command's type. If
`add_todo!` later gains `reject TooLong`, the compiler flags `App.Model`'s
`problem` field and the `match` in `App.view`. Server-side error cases are
checked all the way into the UI.

## 6. Declarations

| Declaration | May read tables | May write | May `reject` | May call externs | Runs where |
|---|---|---|---|---|---|
| `record`, tag type | | | | | data only |
| `table` | | | | | placed by the deployment |
| pure function (`f = \|x\| …`) | | | | | anywhere; inlined |
| `view` | ✓ | | | | where its data is (server, or a client's local copy) |
| `command` | ✓ | ✓ | ✓ | ✓ | where its data lives; also optimistically on clients |
| `client` (`update`, `view`) | through views | | | only through `do` | the client's machine |
| `check` | ✓ | | | | evaluated by the simulator |
| `effect`, `handler` | per operation | | | | section 10 |
| `extern` | | | ✓ | is one | machines whose platform provides its capability |

Writing a table inside a `view` is a compile error, not a runtime surprise.

## 7. Clients

Clients follow the Elm Architecture: a `Model`, a `Msg` union, a pure
`update`, and a pure `view`.

- **`do` turns a command call into a value.** `update` is pure, so it can't
  call `add_todo!` directly. `do add_todo!(m.draft)` builds a command value
  that the platform runs, like Elm's `Cmd`.
- **`else` is required exactly when the command can reject.** `do
  add_todo!(…)` without `else` doesn't compile; `do toggle!(…)` needs none.
- **Views read server data directly.** `view` calls `todo_list(m.show)` as if
  it were local. The compiler *elaborates* the client into a complete one: it
  adds a hidden model field holding the client's local copy of synced data, a
  hidden message for incoming changes, and subscriptions derived from which
  views `view` reads with which arguments. After elaboration, `view` is a pure
  function of the model again.
- **UI markup is part of the language.** Trailing blocks (`ul { … }`) and keyed
  loops (`for t in … key t.id`) are typed against `Msg`, so a misspelled
  message is a compile error.

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
  steps:   500 random msgs per client
}
```

`deploy local` is the reference semantics from section 1: the same program on
one machine, with no distribution at all.

The same source compiles to different plans under different hints or data
sizes. For a personal todo list, replicating all of `todos` to the browser and
evaluating views locally is cheapest: filters switch instantly and the app
works offline. For a million rows, running views on the server and pushing only
results is cheapest.

## 9. Escape hatches

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

## 10. User-defined effects

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

## 11. Compilation pipeline

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
| Views | A SQL query plus a change feed, or local TypeScript evaluation if `todos` is replicated |
| `App` | TypeScript on a small Elm-Architecture runtime; Elm as a second backend |
| `table todos` | Schema, migrations, and indexes inferred from use (`sort_by(.created)`) |
| `!` effects | Calls into each platform; the sim platform runs them deterministically from a seed |

## 12. Testing

- **`check`** declarations are domain invariants, evaluated by the simulator.
- **Convergence is always checked.** Once sync settles, every client's views
  must equal the server's. Nobody writes this check; the compiler derives it.
- **`simulate`** blocks drive many simulated clients with random messages,
  network faults and offline periods. Because `update` and `view` are pure and
  every effect is declared, a failing seed replays exactly.

## 13. Front ends

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

## 14. Open decisions

1. **Reactive reads.** Unmarked, as proposed, or marked with `!` like every
   other effect, as Roc would do? *Recommendation:* unmarked, so views stay
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

## 15. Prior art

Clones live in `refs/` (see [refs/README.md](../refs/README.md)).

| Project | What weft takes from it |
|---|---|
| Roc (`refs/roc`) | Platforms, `!`/`=>` purity inference, structural tag unions |
| Koka (`refs/koka`) | Effect rows, handlers, operation kinds, Perceus |
| Elm | The client architecture |
| JAX (`refs/jax`) | Small IR, primitives with per-transformation rules, sharding annotations |
| Hydro (`refs/hydro`) | Typed locations, lattices, a compiler stack for distributed programs |
| Wing (`refs/wing`) | Abstract resources with one implementation per target, including a simulator |
| TigerBeetle, Maelstrom (`refs/tigerbeetle`, `refs/maelstrom`) | Deterministic simulation, checking against consistency models |
| Replicache / Zero | Optimistic commands that run on both client and server |
