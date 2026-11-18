---
title: Commands and queries
description: Remote functions the client calls on the server, and the guarantees weft gives about them.
---

# Commands and queries

The client and the server are one program, and they talk through **remote
functions**: server functions the client calls as if they were local. A
**query** reads; a **command** writes. Both are declared explicitly, as
values, because together they're the app's API: the one place where the
client can reach the server.

```weft
items_in = Query(|list: Id(TodoList), show: Filter| -> List(Item) {
  items
    .filter(|i| i.list == list and keeps(show, i))
    .sort_by(.created)
})

add_item = Command(|list: Id(TodoList), title: Text| -> Result(Id(Item), [EmptyTitle, NoSuchList]) {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  if not lists.has(list) { return Err(NoSuchList) }
  Ok(items.insert(Item { list, title: cleaned }))
})
```

- **`Query(f)` and `Command(f)`** wrap an ordinary function. The parameter
  types and the result type must be written: they're the contract between
  client and server.
- **They run on the server.** Client code calls them; it never reads or writes
  shared tables itself (see [Server-only code](#server-only-code)).
- **They're the whole API.** `weft explain api` lists every one, and each
  build compares the list with the deployed version, so removing or changing a
  remote function that older clients still call is reported.

## Guarantees

Each of these removes a class of bug common in apps built on server actions or
remote functions:

| Guarantee | The bug it removes |
|---|---|
| **End-to-end types.** Arguments and results are checked on both sides, and arguments are validated against their types when they arrive. | Client and server disagreeing on a shape; unvalidated input |
| **Failures are typed.** A command that can fail returns a `Result`, its reasons are part of its type, and a `Result` can't be ignored. | Errors the UI never handles |
| **Writes refresh reads.** When a command writes a table, every query the client is showing that reads that table is refreshed. | Stale screens after a write, from forgetting to invalidate |
| **Retries apply once.** Every command call carries a request id, and the server applies each id at most once. | Double submits; a retried request applied twice |
| **Commands are atomic.** A command takes effect completely or not at all, and runs as if no other command ran at the same time. | Lost updates and races between concurrent requests |
| **Queries compile to one query.** A query's table reads become one database query, or one per level of nesting in its result, or a compile error. | N+1 queries |
| **Server-only stays on the server.** Tables, `Secret`s and server-only modules can't be referenced from client code. | Leaking data or secrets into the browser bundle |
| **API changes are reported.** Each build compares the API with the deployed one. | Old clients calling a function that changed |

Who may call each remote function is the authorization question, which is
still open (see [the language design](/design/language)). Until it's
designed, every client can call every query and command.

## Commands

A **command** is a `Command(…)`: a server function that may write tables.

- **Atomic.** Either all of its writes happen or none do, and no other
  command's writes appear partway through. How that's achieved on a given
  database is the compiler's job, and `weft explain` shows it.
- **Its result** is the value of its last line, here `Ok` with the new item's
  id.
- **Commands can call other functions,** including other commands. A command
  called from a command is an ordinary call inside the same atomic run.
- **A command can't wait on the outside world.** Calls to other services
  (`http`, `ws`), `sleep` and `.await` are compile errors in a command,
  because it's atomic and may be retried. Fetch first, in a handler, task or
  job, and pass the result in; or record what needs doing in a table and let
  a [job](resources#jobs) do it.

### Failing

A command that can fail returns a [`Result`](types#option-and-result). The
error type lists the reasons, and callers must handle every one.

- **Returning `Err` undoes the command.** None of its writes are kept, and the
  error goes to the caller.
- **Nested commands** follow the same rule: if a command called from another
  returns `Err`, its own writes are undone, and the caller decides what to do
  next. `?` passes the error on:

  ```weft
  move_item = Command(|id: Id(Item), to: Id(TodoList)| -> Result({}, [NoSuchItem, EmptyTitle, NoSuchList]) {
    let item = items.get(id).ok_or(NoSuchItem)?
    add_item(to, item.title)?
    items.delete(id)
    Ok({})
  })
  ```

  If `add_item` fails, `move_item` returns its error, and nothing either of
  them wrote is kept.

- **A command that can't fail** returns its value directly, without `Result`.
- Reasons are tags, with optional payloads:
  `Err(TooManyLists({ limit: 100 }))`.

## Queries

A **query** is a `Query(…)`: a server function that only reads.

- **Table reads compile to database queries,** with a guarantee: one query,
  or one per level of nesting, never one per row, or the compiler reports why
  it can't. See [Data: queries on tables](data#queries-on-tables).
- **A query reads one snapshot.** All of its reads see the data as of one
  moment.
- A query that writes a table, sends a command or reads the clock is a
  compile error.

### Using a query in a component

A component calls a query like a function, in its `view` or bindings:

```weft
ul {
  for i in items_in(list.id, show) { ItemRow(item: i) }
}
```

- **Results are cached by arguments.** Every component on a client that calls
  `items_in(list.id, All)` shares one request and one result.
- **Results refresh when they may have changed:** when the arguments change,
  and when a command writes a table the query reads.
- **While a refresh is on its way, the old result stays on screen.** A
  component isn't shown until each query it reads has a first result; after
  that, it never goes blank while waiting. To show loading or refreshing
  states, use [`status`](ui#loading-states).
- Each query result is its own snapshot. Two queries on one screen may come
  from slightly different moments, and refreshing brings them together.

How the client cache behaves beyond this (keeping results across pages,
prefetching, how long an unused result is kept) is still being designed.

## Calling commands from a client

In a component, calling a command *sends* it, and the handler carries on.
[`.await`](ui#waiting-await) waits for the server's result:

```weft
create = || {
  problem = None
  let pending = name
  name = ""
  match create_list(pending).await {
    Ok(id)   -> go(ListPage(id: id))
    Err(err) -> problem = err
  }
}
```

- **The screen updates when the server confirms:** the queries the command
  affects are refreshed. Showing a change before the server confirms
  (optimistic updates) is part of the client cache design, still open.
- **A command that can't fail** can be sent without waiting:
  `on_change: |d| set_done(item.id, d)`.
- **A command that can fail must be awaited and its result handled,** because
  a `Result` can't be ignored. `.on_err(f)` handles just the failure:

  ```weft
  component ItemTitle(item: Item) {
    state draft   = item.title
    state problem = None

    save = || rename_item(item.id, draft)     // error: rename_item can fail with [EmptyTitle]; await it and handle the Result
  }
  ```

  The fix: `save = || rename_item(item.id, draft).await.on_err(|err| problem = err)`.

- While the handler waits, other handlers run. If the component goes away
  first, the handler is cancelled, but the command still runs.

What it compiles to: an endpoint and a typed client call, argument
validation, and a request id so a retried send is applied once.

## Server-only code

Tables, `Secret`s, externs and the server's platform modules exist only on
the server. Client code (components, and functions only they call) reaches
them through queries and commands:

```weft
component Count {
  view = p { "${items.count()} items" }    // error: `items` is a server table; read it in a Query and call that
}
```

The compiler knows which functions run where from what they reference, so a
helper used on both sides is compiled for both, and one that reads a table
can only be used on the server.
