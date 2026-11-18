---
title: Commands and queries
description: Functions that change data and functions that read it, and how clients use them.
---

# Commands and queries

## Commands

A **command** is a `!` function in `commands/` that writes tables. Clients
call commands to change data.

```weft
add_item! : Id(TodoList), Text => Id(Item) ! <reject [EmptyTitle, NoSuchList], ..>
add_item! = |list, title| {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  if not lists.has(list) { reject NoSuchList }
  items.insert!(Item { list, title: cleaned })
}
```

- **The signature is required.** Parameter types, return type and `reject`
  reasons are the contract between clients and the server. The rest of the row
  can be left as `..`.
- **A command is a transaction.** It takes effect completely or not at all. It
  sees its own writes as it goes, and no other command's writes interleave
  with it.
- **A command is written as if on one machine.** Retries, duplicate requests,
  ordering against other commands and offline replay are added by the
  compiler, from the command's effects (see [The promise](/docs/promise)).
- **Its result** is the value of its last line, here the new item's id.
- **Commands can call other functions,** including other commands. A command
  called from a command is an ordinary call inside the same transaction.

### `reject`

`reject Reason` stops a command, undoes everything it did, and reports
`Reason` to the caller.

```weft
if cleaned.is_empty() { reject EmptyTitle }
if lists.count() >= 100 { reject TooManyLists({ limit: 100 }) }
```

- Reasons are tags, with optional payloads. They become part of the command's
  type, and callers must handle every one.
- `reject` only works in commands and functions they call. Pure code returns a
  [`Result`](types#option-and-result) instead.
- What it compiles to: the check runs on the client first, for instant
  feedback, and again on the server, where it's authoritative.

## Queries

A **query** is a function in `queries/` that reads tables and writes nothing.
Views use queries to show data.

```weft
items_in = |list, show| items
  .filter(|i| i.list == list and keeps(show, i))
  .sort_by(.created)
```

- **Views stay in sync.** When a view reads a query, the client subscribes to
  it. When the rows it read change, on this device or another, it's
  recomputed and the view updates.
- **No signature is required.** Queries aren't called across the network the
  way commands are; the compiler decides where each one runs.
- What it compiles to: a database query plus a change feed on the server, or a
  computation over the client's local copy of the tables. The compiler
  chooses per query, from the data's size and the deployment.

## Calling commands from a client

In a component, calling a command *sends* it. The call doesn't block, and the
handler carries on.

```weft
create = || {
  problem = None
  create_list!(name)
    then |id| go(ListPage(id: id))
    else |err| problem = err
  name = ""
}
```

- **The command applies optimistically:** the client runs it against its local
  copy straight away, so the change shows instantly. The server then confirms
  it, or the client undoes it.
- **`else |reason| …` is required exactly when the command can reject.** It
  runs if the server rejects the command, with the component's state at that
  moment. If the component has gone by then, it's skipped, but the
  optimistic change is still undone.
- **`then |result| …`** is optional. It runs when the server confirms, with the
  command's result.
- **Both handlers are small handlers themselves:** they can set state, send
  more commands and navigate.
- Leaving out a required `else` is a compile error:

  ```weft
  save = || rename_item!(item.id, draft)     # error: rename_item! can reject [EmptyTitle]; add `else |err| …`
  ```

What it compiles to: an endpoint and a typed client call, a request id so a
retried send is applied once, and, for offline clients, a queue replayed in
order when the connection returns.
