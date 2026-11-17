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

```text
todo-lists/
  src/
    data.weft     records and tables
    logic.weft    commands (every write) and views (every read)
    app.weft      the browser client
    checks.weft   invariants: the project's only tests
  deploy.weft     where it runs, and simulations
```

Everything in `src/` is written as if it ran on one machine. Only
`deploy.weft` knows there's more than one.

## Data

```weft
record Item {
  key id:  Id   = new_id!()    # `!`: generating an id is an effect
  list:    Id
  title:   Text
  done:    Bool = False
  created: Time = now!()
}

Filter : [All, Active, Done]   # a tag union

table items : Item             # a table of Items, placed by deploy.weft
```

## Commands: every write

```weft
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

## Pure functions and views: every read

```weft
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}

view items_in(list: Id, show: Filter) -> List(Item) =
  items
    .filter(|i| i.list == list and keeps(show, i))
    .sort_by(.created)
```

- `|show, i|` is a function. `match` must handle every case.
- A `view` may read tables but never write them. Clients subscribe to views,
  and the compiler decides whether to compute one on the server or on the
  client's local copy.
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

## The client: the Elm Architecture

```weft
client App {
  Model : { selected: [None, Some(Id)], item_draft: Text, problem: [None, EmptyTitle, NoSuchList] }
  Msg   : [Select(Id), TypedItem(Text), AddItem, SetDone(Id, Bool), Rejected(_)]

  update = |m, msg| match msg {
    TypedItem(t)   -> ({ ..m, item_draft: t, problem: None }, none)
    AddItem        -> match m.selected {
                        None     -> (m, none)
                        Some(id) -> ({ ..m, item_draft: "" }, do add_item!(id, m.item_draft) else Rejected)
                      }
    SetDone(id, d) -> (m, do set_done!(id, d))
    Rejected(err)  -> ({ ..m, problem: err }, none)
    Select(id)     -> ({ ..m, selected: Some(id) }, none)
  }

  view = |m| main {
    match m.selected {
      None     -> p { "Pick a list" }
      Some(id) -> ul {
        for i in items_in(id, All) key i.id {
          li {
            checkbox(checked: i.done, on_change: SetDone(i.id, not i.done))
            span { i.title }
          }
        }
      }
    }
  }
}
```

- `update` is pure. `do add_item!(…)` doesn't run the command; it returns it
  for the runtime to run.
- `else Rejected` is required because `add_item!` can reject. Leave it out and
  the program doesn't compile.
- `{ ..m, item_draft: t }` is a copy of `m` with one field changed.
- `view` reads `items_in(…)` as if it were local. The compiler adds the
  syncing.

## Checks: the only tests you write

```weft
check titles_not_blank = items.all(|i| not i.title.trim().is_empty())
check no_orphans       = items.all(|i| lists.has(i.list))
```

The simulator evaluates these after every step of every simulated run. Two
more checks are always on: every run must match some one-machine run, and
clients must converge with the server.

## Deployment and simulation

```weft
deploy local {
  machine laptop on "weft/local"     # the reference: one process, in memory
  place everything on laptop
}

deploy prod {
  machine server  on "weft/server-rust-postgres"
  machine browser on "weft/browser-ts" { count: many, offline: True }
  place lists, items on server
  place App          on browser
  consistency lists, items: session
}

simulate flaky_wifi {
  topology: prod
  platform: "weft/sim"
  clients:  3
  network:  drop 10%, latency 5ms..2s, offline 30%
  steps:    500 random msgs per client
}
```

## Escape hatches

Code weft can't see into is declared as an `extern`. It must name the
capability it needs and say how it behaves in simulation:

```weft
extern send_email! : Text, Text => {} ! <reject [Bounced]>
  needs email
  sim { ok 95%, else Bounced, latency 50ms..2s }
```
