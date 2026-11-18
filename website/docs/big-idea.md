---
slug: /
sidebar_label: The big idea
description: Write the system once, as if it ran on one machine. That program is the spec.
---

# The big idea

:::note[A design, not yet a language]
Nothing on this site compiles yet. These pages describe what weft is designed
to do. [Status](status) says what exists today.
:::

**Write the system once, as if it ran on one machine. That program is the
specification. weft builds the running version (a server with its database,
browser clients, and the typed calls between them) and checks it against the
specification.**

Everything else on this page follows from that one idea.

## The status quo

A small app that works across devices is usually written several times over,
in different shapes:

- a database schema;
- server endpoints;
- client code that calls them;
- validation, cache invalidation and retries;
- tests for each piece.

Most bugs live in the gaps between those pieces, and most tests check one
piece at a time.

## Consequences

### 1. The API is a list of functions

**Today:** routes, request handlers, fetch calls, serialization, and client
types kept in step with server types by hand.

**In weft:** the server's API is its queries and commands, written as
ordinary functions wrapped in `Query(…)` and `Command(…)`:

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

The client calls them as if both were local:

```weft
add = || {
  let title = draft
  draft = ""
  add_todo(title).await.on_err(|err| problem = err)
}
```

```weft
ul { for t in todo_list(show) { TodoRow(todo: t) } }
```

The compiler is meant to generate the endpoint, the typed client call,
argument validation, and a request id so a retried call is applied once.
When `add_todo` writes `todos`, every `todo_list` on screen is refreshed.

### 2. The machines are a separate file

**Today:** which machines exist, and what runs on each, is spread through
build scripts, environment variables and code.

**In weft:** the logic lives in `data/`, `commands/`, `queries/` and `ui/`. A
deployment, in its own file, says which machines exist and what platform each
runs on:

```weft
prod = Deployment {
  machines: {
    server:  Machine { platform: "weft/server-rust-postgres" },
    browser: Machine { platform: "weft/browser-ts", count: Many },
  },
}
```

The shape is fixed: one server with a database, and clients. Tables, queries
and commands run on the server; components run on the clients. Where data
lives beyond that (copies on clients, offline use, several servers) is
deliberately not designed yet. See [the language design](/design/language).

Every project also has a deployment called `local`: one process in memory,
with no network at all. It's generated, never written, and it's the reference
every other deployment is checked against.

### 3. Effects are visible

**Today:** any function can do I/O, so a signature doesn't tell you what a
function touches.

**In weft:** the compiler works out exactly what every function touches,
from its code, without your writing it down:

```text
$ weft explain effects
add_todo     writes todos; random; time
todo_list    reads todos
```

The compiler uses what it inferred to decide which queries a command
refreshes, which code can't reach the client, and which commands must not
interleave. Your editor shows it on hover.

### 4. Races are found before users find them

**Today:** races between devices are rare in testing and common in
production.

**In weft:** a deterministic simulator runs many clients against the server,
with dropped messages, delays, periods without the server, and crashes. It
checks every run against the one-machine program. When a run fails, its seed
replays it exactly.

For example: your phone adds "milk" to the Groceries list while your laptop
deletes Groceries. On one machine the two commands run one after the other:
either "milk" is deleted with the list, or adding it fails with `NoSuchList`.
A typical backend can get this wrong even with one database. At Postgres's
default isolation, the add's check can see the list while the delete is
still committing, and "milk" is stored in a list that no longer exists. weft
promises that every command runs as if alone, so the compiler must rule this
out, and the simulator is designed to find exactly this kind of difference.

### 5. The guarantees come with the language

**Today:** server actions and remote functions make calling the server easy,
and leave the rest to you: validating input, invalidating caches after a
write, making retries safe, avoiding one query per row.

**In weft:** each of those is a guarantee: calls are typed and validated,
failures are typed and can't be ignored, writes refresh the queries that read
what was written, retries are applied once, commands are atomic, queries
compile to one database query per level of nesting, and server-only code
can't reach the client. See
[Commands and queries: guarantees](/docs/reference/commands-and-queries#guarantees)
and [The promise and its limits](promise).

### 6. The target can change without the program changing

**Today:** switching databases or clouds is a rewrite.

**In weft:** *platforms* provide the effects: storage, network, the DOM.
Moving to a different platform changes a file in `deploy/`, not the rest of
the project. JAX works the same way: one numerical program, compiled for
CPUs, GPUs or TPUs.

## What it isn't

- **It isn't distributed yet.** The design commits to one server with a
  database, and clients. Copies of data on clients, offline writes and
  consistency levels are left open on purpose: some invariants can't be kept
  there without devices coordinating first, and weft won't pick an answer
  before its consequences are worked out. See
  [The promise and its limits](promise).
- **It isn't built.** The design, an example app, an editor extension and
  these docs exist; the compiler and simulator don't. See [Status](status).
- **The front end isn't settled.** The current design is a new language. It
  might instead start embedded in an existing one. See
  [Why doesn't weft…?](why-not#why-a-new-language-instead-of-a-library).
