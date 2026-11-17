---
sidebar_label: Weft at a glance
description: The whole language on one page, through the todo-lists example.
---

# Weft at a glance

The whole language on one page, with each piece taken from the todo-lists
example in the repo (`examples/todo-lists`). The full design is in the
[language proposal](/design/language).

:::note[A design, not yet a language]
This is the proposed syntax. Nothing here compiles yet, and details will
change.
:::

## A project's files

Every project has the same layout, in the spirit of Rails:

```text
todo-lists/
  weft.toml        the project's name
  data/            records and their tables, one record per file
  commands/        every write
  queries/         every read the client uses
  ui/
    App.weft       the client: the layout around every page
    pages/         one page per URL: lists/[id].weft is /lists/:id
    components/    one component per file
  checks/          invariants: the project's only tests
  deploy/          where it runs
  simulate/        simulated runs
```

Everything outside `deploy/` and `simulate/` is written as if it ran on one
machine. There are no `module` lines: the project is the module.

## Data

```weft title="data/item.weft"
record Item {
  key id:  Id   = new_id!()    # `!`: generating an id is an effect
  list:    Id
  title:   Text
  done:    Bool = False
  created: Time = now!()
}

table items : Item             # tables are always declared; weft never guesses plurals
```

```weft title="data/filter.weft"
Filter : [All, Active, Done]   # a tag union
```

## Commands: every write

```weft title="commands/items.weft"
command add_item!(list: Id, title: Text) {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  if not lists.has(list) { reject NoSuchList }
  items.insert!(Item { list: list, title: cleaned })
}
```

- A command is a transaction: it takes effect entirely or not at all.
- `!` marks anything that changes the world. Here that's the command itself
  and `insert!`.
- `reject` stops the command with a reason. The reasons become part of its
  type, and callers must handle them.

## Queries: every read

```weft title="queries/items.weft"
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}

query items_in(list: Id, show: Filter) -> List(Item) =
  items
    .filter(|i| i.list == list and keeps(show, i))
    .sort_by(.created)
```

- `|show, i|` is a function. `match` must handle every case.
- A `query` may read tables but never write them. Clients subscribe to
  queries, and the compiler decides whether to compute one on the server or on
  the client's local copy.
- `.created` is shorthand for `|i| i.created`.

## Effects in types

You rarely write these; the compiler infers them and your editor shows them.

```weft
add_item! : Id, Text => {}  ! <write items, read lists, reject [EmptyTitle, NoSuchList], random, time>
items_in  : Id, Filter -> List(Item)  ! <read items>
keeps     : Filter, Item -> Bool
```

- `=>` means the function changes the world, and `->` means it doesn't.
- The row after `!` says exactly what it touches. `random` and `time` are
  there because a new `Item` gets a fresh id and a timestamp.

## The client: components, state and actions

```weft title="ui/components/ItemRow.weft"
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
      span(on_click: edit) { item.title }
    }
    match problem {
      None       -> nothing
      EmptyTitle -> p(class: "error") { "An item needs a title" }
    }
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
}
```

- `state` is local to this component: each row has its own `editing` and
  `draft`. Types are inferred. `problem` is `[None, EmptyTitle]` because that's
  what `rename_item!` can reject with.
- `view` is markup and can't change anything. Event attributes name an
  action: `on_click: edit`, or `on_change: done(not item.done)` with an
  argument.
- An `action` is the only place state changes. `draft = item.title` sets
  state; `do` sends a command once the action finishes.
- `else |err| …` is required because `rename_item!` can reject. Leave it out
  and the program doesn't compile.
- `input(bind: draft)` keeps the input and the state in step, with no action
  per keystroke.

## Pages and the client

Each file in `ui/pages/` is a page, and its path is its URL. A page is a
component whose parameters come from the URL:

```weft title="ui/pages/lists/[id].weft"
page ListPage(id: Id) {
  view = match find_list(id) {
    Missing     -> p { "This list was deleted on another device" }
    Found(list) -> ListPanel(list: list)
  }
}
```

The `client` is the layout around every page. `outlet` marks where the current
page goes:

```weft title="ui/App.weft"
client App {
  view = main {
    Sidebar
    outlet
  }
}
```

Links name a page and its parameters, so a broken link is a compile error:

```weft
link(to: ListPage(id: row.list.id)) { row.list.name }
```

Views read server data (`find_list(id)`, `items_in(…)`) as if it were local;
the compiler adds the syncing. Underneath, each component compiles to the Elm
Architecture: every action becomes a message, and the assignments become a
pure state update.

## Checks: the only tests you write

```weft title="checks/items.weft"
check titles_not_blank = items.all(|i| not i.title.trim().is_empty())
check no_orphans       = items.all(|i| lists.has(i.list))
```

The simulator evaluates these after every step of every simulated run. Two
more checks are always on: every run must match some one-machine run, and
clients must converge with the server.

## Deployment and simulation

```weft title="deploy/prod.weft"
deploy prod {
  machine server  on "weft/server-rust-postgres"
  machine browser on "weft/browser-ts" { count: many, offline: True }

  consistency lists, items: session
}
```

```weft title="simulate/flaky_wifi.weft"
simulate flaky_wifi {
  clients: 3
  network: drop 10%, latency 5ms..2s, offline 30%
  steps:   500 random actions per client
}
```

Conventions fill in the rest:
- `deploy local`, the one-process reference, is generated.
- Tables go on the machine named `server`, and the client on `browser`.
- Simulations run `prod`'s layout on simulated platforms.
- `weft explain` lists every default it applied.

## Escape hatches

Code weft can't see into is declared as an `extern`. It must name the
capability it needs and say how it behaves in simulation:

```weft
extern send_email! : Text, Text => {} ! <reject [Bounced]>
  needs email
  sim { ok 95%, else Bounced, latency 50ms..2s }
```
