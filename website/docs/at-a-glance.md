---
sidebar_label: Weft at a glance
description: The whole language on one page, through the todo-lists example.
---

# Weft at a glance

The whole language on one page, with each piece taken from the todo-lists
example in the repo (`examples/todo-lists`). Every section links to its page
in the [Reference](/docs/reference/overview).

:::note[A design, not yet a language]
This is the designed syntax. Nothing here compiles yet.
:::

## A project's files

Every project has the same layout, in the spirit of Rails
([Projects](/docs/reference/projects)):

```text
todo-lists/
  weft.toml        the project's name
  data/            records and their tables
  commands/        functions that write: what clients call
  queries/         functions that read: what views use
  ui/
    App.weft       the root component, around every page
    pages/         one component per URL: lists/[id].weft is /lists/:id
    components/    one component per file
  checks/          invariants: the project's only tests
  deploy/          where it runs
  simulate/        simulated runs
```

There are no `module` lines and no imports: the project is the module. There
are no `command`, `query` or `check` keywords either: what a function is
follows from what it does and which folder it's in.

## Data

```weft title="data/item.weft"
## One item on a list.
record Item {
  key id:  Id(Item)     = new_id!()   # `!`: making an id is an effect
  list:    Id(TodoList)               # a reference to a row of lists
  title:   Text
  done:    Bool = False
  created: Time = now!()
}

items = Table(Item, delete_with: { list: lists })
```

- A `record` is a type; `key` marks its primary key.
- A table is a value: `Table(Item, …)`, with options such as deleting items
  along with their list. See [Data and tables](/docs/reference/data).

## Commands: every write

```weft title="commands/items.weft"
add_item! : Id(TodoList), Text => Id(Item) ! <reject [EmptyTitle, NoSuchList], ..>
add_item! = |list, title| {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  if not lists.has(list) { reject NoSuchList }
  items.insert!(Item { list, title: cleaned })
}
```

- A function in `commands/` that writes is a command: clients call it, and it
  runs as a transaction.
- Its signature is required, because it's the API: parameter types, result,
  and the `reject` reasons. `..` stands for the rest of its effects.
- `!` marks anything that changes the world.
- `reject` stops the command and undoes it. See
  [Commands and queries](/docs/reference/commands-and-queries).

## Queries: every read

```weft title="queries/items.weft"
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}

expect keeps(Active, { done: True }) == False

items_in = |list, show| items
  .filter(|i| i.list == list and keeps(show, i))
  .sort_by(.created)
```

- A function is `name = |parameters| body`. `match` must cover every case.
- A read-only function in `queries/` is a query. Views that use it stay in
  sync as the data changes, on this device or another.
- `expect` states a fact about pure code; `weft test` checks it.

## The client: components

```weft title="ui/components/ItemRow.weft"
component ItemRow(item: Item) {
  state editing = False
  state draft   = ""
  state problem = None

  edit = || {
    editing = True
    draft = item.title
  }

  save = || {
    editing = False
    rename_item!(item.id, draft) else |err| problem = err
  }

  view = li {
    checkbox(checked: item.done, on_change: |d| set_done!(item.id, d))
    if editing {
      form(on_submit: save) {
        input(bind: draft)
        button(on_click: || editing = False) { "Cancel" }
      }
    } else {
      span(class: { done: item.done }, on_click: edit) { item.title }
    }
    match problem {
      None       -> nothing
      EmptyTitle -> p(class: "error") { "An item needs a title" }
    }
  }

  style {
    .done { text-decoration: line-through }
  }
}
```

- **`state`** is local to each instance: every row has its own `editing`.
  Types are inferred: `problem` is `[None, EmptyTitle]` because that's what
  `rename_item!` can reject with.
- **Handlers** are ordinary functions in the component that change its state,
  named (`edit`) or inline (`|| editing = False`). They're the only place
  state changes.
- **Calling a command** in a handler sends it. `else |err| …` is required
  because `rename_item!` can reject; `then |result| …` is optional.
- **`view`** is markup, and can't change anything. **`style`** is CSS scoped to
  this component.

Underneath, each component compiles to the Elm Architecture: every handler
becomes a message, and the assignments become one pure update function. See
[UI](/docs/reference/ui).

## Pages

```weft title="ui/pages/lists/[id].weft"
component ListPage(id: Id(TodoList)) {
  view = match find_list(id) {
    None       -> p { "This list was deleted on another device" }
    Some(list) -> ListPanel(list: list)
  }
}
```

```weft title="ui/App.weft"
component App {
  view = main {
    Sidebar
    outlet
  }
}
```

- A file's path under `ui/pages/` is its URL, and a page's parameters come
  from the URL.
- `ui/App.weft` wraps every page; `outlet` marks where the page goes.
- Links and navigation are typed: `link(to: ListPage(id: x)) { … }`, and
  `go(Home)` in a handler.

## Checks: the only tests you write

```weft title="checks/items.weft"
titles_not_blank = items.all(|i| not i.title.trim().is_empty())
no_orphans       = items.all(|i| lists.has(i.list))
```

A `Bool` in `checks/` is an invariant. The simulator checks every one after
every step of every simulated run. Two more are always on: every run must
match some one-machine run, and clients must converge with the server. See
[Checks and tests](/docs/reference/checks-and-tests).

## Deployment and simulation

```weft title="deploy/prod.weft"
prod = Deployment {
  machines: {
    server:  Machine { platform: "weft/server-rust-postgres" },
    browser: Machine { platform: "weft/browser-ts", count: Many, offline: True },
  },
  consistency: { lists: Session, items: Session },
}
```

```weft title="simulate/flaky_wifi.weft"
flaky_wifi = Simulation {
  clients:            3,
  actions_per_client: 500,
  network:            Network { drop: 10%, latency: 5ms..=2s, offline: 30% },
}
```

- Deployments and simulations are ordinary values, checked like any other
  code.
- Conventions fill in the rest: the one-process `local` deployment is
  generated, tables go on `server`, and the client on `browser`. See
  [Deployment and simulation](/docs/reference/deployment).

## Escape hatches

```weft title="externs/send_email.weft"
send_email! : Text, Text => {} ! <reject [Bounced]>
send_email! = Extern {
  needs: [Email],
  sim:   Fallible { ok: 95%, error: Bounced, latency: 50ms..=2s },
}
```

Code weft can't see into is an extern: a full signature, the capabilities it
needs, a simulation model, and host code beside it. See
[Externs, jobs and endpoints](/docs/reference/resources).
