---
title: Effects
description: What the compiler infers about every function, where effects are allowed, snapshots, and user-defined effects and handlers.
---

# Effects

An *effect* is anything a function does besides compute its result from its
arguments: reading or writing a table, reading the clock, sending a command.
weft follows Koka: effects are tracked precisely, but by the compiler, not in
your code. Nothing marks them in names or types.

## Inferred, never written

The compiler works out every function's effects from its body, including
everything it calls:

| Effect | Comes from |
|---|---|
| Reads a table | `items.filter(…)`, `lists.get(id)` |
| Writes a table | `items.insert(…)`, `items.update(…)` |
| Reads the clock | `now()`, `today()` |
| Randomness, including new ids | `new_id()`, `random(range)` |
| Assigns component state | `draft = ""` in a handler |
| Sends a command | calling one from a component |
| Navigates | `go(Home)` |
| Reads a query's loading status | `status(…)` |
| Calls the outside world | `http.get(url)`, `ws.connect(url)`, `sleep(1s)` |
| Calls an extern | `send_email(to, body)` |
| Uses a user-defined effect | `Audit.note("…")` |

A function with none of these is **pure**: the same arguments always give the
same result.

You never write effects down. Your editor shows them on hover and can
highlight calls that read or write, and `weft explain` prints them:

```text
$ weft explain effects
add_item     reads lists; writes items; random; time
items_in     reads items
keeps        pure
ListPanel    calls items_in; sends add_item, rename_list, delete_list, clear_done
```

The compiler uses this for the guarantees that depend on it: a command that
writes `items` refreshes the queries that read `items`; code that reads a
server table can't end up in the client; and `add_item`, which reads `lists`,
must not interleave with `delete_list`, which deletes from it (see
[The promise](/docs/promise)).

### Effects follow their callers and callbacks

- **A function has the effects of everything it calls.** If `add_item` builds
  an `Item`, whose `id` defaults to `new_id()`, then `add_item` uses
  randomness.
- **Higher-order functions take on their callbacks' effects.** `list.map(f)`
  is pure when `f` is, and reads `items` when `f` does. There's one `map`,
  not a pure one and an effectful one.

## Where effects are allowed

Most code can do anything; the compiler only needs effects to be allowed where
they make sense. These are compile errors, and each names the effect and where
it comes from:

| Place | Allowed | Example error |
|---|---|---|
| `expect` | pure code only | `expect can't read the clock: stamp calls now()` |
| Top-level constants | pure code only, since they're computed once | `total can't read items: a top-level constant is computed once. Make it a function: total = \|\| items.count()` |
| A component's `view` and its other bindings | calling queries, reading `Device` and `Memory` tables | `view can't send a command: only handlers can change data` |
| A [check](checks-and-tests) | reading tables | `no_orphans can't send a command: a check only reads` |
| A function passed to a table method | the [query fragment](data#queries-on-tables) | `this filter can't run as a query: fancy_score uses a while loop` |
| A `Query` | reading tables | `items_in can't write items: a query only reads` |
| A `Command` | everything except the outside world and `.await`, since it's atomic and may be retried | `add_item can't call http.get: a command runs atomically and may be retried. Fetch first and pass the result in` |
| Client code | everything except shared tables, `Secret`s and server-only modules | `count can't read items: it's a server table. Read it in a Query` |
| Handlers, tasks, jobs, endpoints, and anything else | everything | |



## Snapshots

Reading a table is an effect because data changes over time: the same read
at two moments can give two answers. Within one run of a function, though,
every read is consistent:

- **A command** reads one snapshot of the data, plus its own writes. It's
  atomic and runs as if alone: no other command's writes appear partway
  through.
- **A handler or task** reads one snapshot until it reaches an `.await`, and
  a newer one after.
- **A query** reads one snapshot. Each query result on a client is its own
  snapshot, so two queries on one screen may come from slightly different
  moments until their next refresh.
- **A component is shown once its data is ready:** not until every query it
  calls has a first result. So when `find_list(id)` returns `None`, the list
  doesn't exist. It never means "not loaded yet". To show loading states, use
  [`status`](ui#loading-states).
- **In a simulation,** the simulator chooses when each request and response
  arrives, from the run's seed. That's what lets a run replay exactly,
  and what the [checks](checks-and-tests) are tested against.

## Built-in effectful functions

| Function | Effect | Returns |
|---|---|---|
| `now()` | reads the clock | the current `Time` |
| `today()` | reads the clock | the current `Date` |
| `new_id()` | randomness | a fresh `Id(t)`, whose type comes from context |
| `random(range)` | randomness | a random number in the range |
| `sleep(duration)` | waits | `{}`, after `duration`; use with `.await` |
| `go(route)` | navigates | `{}`; see [UI: pages](ui#pages-and-routes) |
| `status(query)` | reads loading status | see [UI: loading](ui#loading-states) |

Every platform provides these. In a simulation, time and randomness come from
the run's seed, so a run replays exactly. Table methods are in
[Data](data#reading), and the modules for the outside world (`http`, `ws`,
`dom`, `browser`, `secrets`) in [Standard library](stdlib#platform-modules).

## User-defined effects

You can declare your own effect: a set of operations that a *handler*
provides. The code using the effect doesn't know or care which handler is
installed.

```weft
effect Audit {
  note : Text -> {}
}

clear_done : Id(TodoList) -> {}
clear_done = |list| {
  items.delete_where(|i| i.list == list and i.done)
  Audit.note("cleared done items in ${list}")
}

handler audit_to_table : Audit {
  note = |event| audit_log.insert(AuditEntry { event })
}
```

- Inside an `effect`, each operation is a signature. Calling one
  (`Audit.note(…)`) adds the effect to the caller.
- A `handler` gives each operation an implementation.
- **Installing a handler:** `with audit_to_table { … }` handles the effect for
  the code in the braces. A deployment can install handlers for every command
  with its `handlers` field (see [Deployment](deployment#deployments)).
- **Handling removes the effect.** Code inside `with` no longer has the
  effect it handles; it has the handler's effects instead. A function whose
  only effect is handled this way can be pure.
- An effect with no handler installed where it's used is a compile error.

### Operation kinds

An operation's kind says how its handler may return to the caller, and so
where the handler may run:

| Kind | Declared as | The handler… | May the handler run on another machine? |
|---|---|---|---|
| resume-once (default) | `note : Text -> {}` | returns a value, exactly once | yes: it becomes a request and a response |
| final | `final abort : Text -> a` | never returns; the caller stops | yes: it becomes an abort with a reason |
| control | `ctl choose : List(a) -> a` | may return any number of times | no: it must run on the caller's machine |

Control operations are for things like backtracking search and generators. A
control operation captures the rest of the computation, so it can't travel
over the network. A control handler installed on one machine can't handle an
operation performed on another: the server's handlers serve commands, and the
client's serve components.
