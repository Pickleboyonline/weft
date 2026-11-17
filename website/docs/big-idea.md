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
specification. weft builds the distributed version (a server, browser
clients, storage, and the sync between them) and checks it against the
specification.**

Everything else on this page follows from that one idea.

## The status quo

A small app that works across devices is usually written several times over,
in different shapes:

- a database schema;
- server endpoints;
- client code that calls them;
- cache invalidation, retries and offline handling;
- tests for each piece.

Most bugs live in the gaps between those pieces, and most tests check one
piece at a time.

## Consequences

### 1. There's no API layer to write

**Today:** routes, request handlers, fetch calls, serialization, and client
types kept in step with server types by hand.

**In weft:** the client sends a command and reads a view as if both were
local:

```weft
action add {
  do add_todo!(draft) else |err| problem = err
  draft = ""
}
```

```weft
ul { for t in todo_list(show) key t.id { TodoRow(todo: t) } }
```

The compiler is meant to generate the endpoint, the client code that calls
it, and the sync that keeps the client's copy of `todo_list` fresh.

### 2. Where things run is a separate decision

**Today:** the architecture is spread through the code. Moving logic from
server to client, or adding offline support, means rewriting it.

**In weft:** the logic lives in `src/`. A separate `deploy.weft` says which
machines exist, where each table lives, and how fresh each screen must be:

```weft
deploy prod {
  machine server  on "weft/server-rust-postgres"
  machine browser on "weft/browser-ts" { count: many, offline: True }

  place lists, items on server
  place App          on browser
  consistency lists, items: session
}
```

The same source also has a `deploy local`: one process in memory, with no
network at all. That's the reference every other deployment is checked
against.

### 3. Effects are visible

**Today:** any function can do I/O, so a signature doesn't tell you what a
function touches.

**In weft:** a function that changes the world ends in `!`, and its type says
exactly what it touches:

```weft
add_todo! : Text => {}  ! <write todos, reject [EmptyTitle], random, time>
```

The compiler uses that list to decide where code can run, what has to be
synced, and which commands need coordination.

### 4. Distribution bugs are found before users find them

**Today:** races between devices are rare in testing and common in
production.

**In weft:** a deterministic simulator runs many clients against the server,
with dropped messages, delays and offline periods. It checks every run
against the one-machine program. When a run fails, its seed replays it
exactly.

For example: your phone adds "milk" to the Groceries list while your laptop
deletes Groceries. If the delete reaches the server first, the item is stored
in a list that no longer exists. On one machine that can't happen: the two
commands run one after the other, in some order. The simulator is designed to
find exactly this kind of difference.

### 5. Consistency is a stated choice, with stated costs

**Today:** consistency is whatever the database and caching layer happen to
provide, and you find out what that means when something goes wrong.

**In weft:** you declare it (`consistency lists, items: session`). The
compiler is meant to tell you which of your invariants that choice can't keep.
See [The promise and its limits](promise).

### 6. The target can change without the program changing

**Today:** switching databases or clouds is a rewrite.

**In weft:** *platforms* provide the effects: storage, network, the DOM.
Moving to a different platform changes `deploy.weft`, not `src/`. JAX works
the same way: one numerical program, compiled for CPUs, GPUs or TPUs.

## What it isn't

- **It isn't magic.** Some invariants can't be guaranteed while two devices
  are offline and acting independently. weft's job is to tell you which ones,
  and make you choose, not to pretend. See
  [The promise and its limits](promise).
- **It isn't built.** The design, an example app, an editor extension and
  these docs exist; the compiler and simulator don't. See [Status](status).
- **The front end isn't settled.** The current design is a new language. It
  might instead start embedded in an existing one. See
  [Why doesn't weft…?](why-not#why-a-new-language-instead-of-a-library).
