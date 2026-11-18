---
title: UI
description: Components, state, handlers, markup, styling, pages and routing.
---

# UI

A weft client is a tree of **components**. Each one has its own state, the
functions that change it, its markup, and its styles.

## Components

```weft
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
    .done { text-decoration: line-through; opacity: 0.6 }
  }
}
```

- `component Name(parameters) { … }` declares one. Component names are
  capitalized. Parameters are named, and given by name when it's used:
  `ItemRow(item: i)`.
- One component per file, in `ui/components/`, named after it. Pages and the
  client root follow the same form (see [Pages](#pages-and-routes)).
- A component's body holds, in any order: `state` declarations, ordinary
  bindings, handlers, one `view`, and an optional `style`.

## State

`state name = initial` declares a value that belongs to one instance of the
component and can change.

- **Its type is inferred.** `state problem = None` above gets the type
  `[None, EmptyTitle]` from the assignment `problem = err`, because
  `rename_item!` rejects with `EmptyTitle`. Annotate it when inference can't
  tell: `state selected: Option(Id(Item)) = None`.
- **Only this component's functions can assign it,** with plain `=`. Assigning
  it in `view`, or in a binding that isn't a function, is a compile error.
- **State must be data:** records, tags, lists and the like. It can't hold
  functions. So it can always be logged, replayed and kept across a reload.
- **Each instance has its own.** Two `ItemRow`s have separate `editing` states.
  In a loop, the key decides which state belongs to which row
  ([loops](#loops)).

## Derived values

Any ordinary binding in a component that reads state, parameters or queries
is recomputed whenever they change:

```weft
remaining = items_in(list.id, All).count_if(|i| not i.done)
label = if remaining == 1 { "1 item left" } else { "${remaining} items left" }
```

Nothing has to be declared as "derived": weft is pure, so every such binding
already is.

## Handlers

A **handler** is a function in a component that assigns that component's
state. It runs in response to an event.

- Named: `edit = || { … }`, then `on_click: edit`.
- Inline: `on_click: || editing = False`.
- With an event value: `on_change: |d| set_done!(item.id, d)` receives the
  checkbox's new `Bool`.

Handlers are the only way state changes. Each one runs to completion before
the next event, so a handler never sees half of another one's changes.

What it compiles to: every handler, named or inline, becomes a message, and
all of a component's assignments become one pure update function: the Elm
Architecture. That's what lets the simulator replay any run.

### Calling commands

Inside a handler, calling a `!` command sends it once the handler finishes.
`then` and `else` attach handlers for its result. See
[Commands and queries](commands-and-queries#calling-commands-from-a-client).

### Passing handlers to children

A component can take a handler as a parameter, so a child can tell its parent
something:

```weft
component Picker(options: List(Text), on_pick: Action(Text)) {
  view = ul {
    for o in options { li(on_click: || on_pick(o)) { o } }
  }
}
```

The parent passes one of its own handlers: `Picker(options: names, on_pick:
|n| chosen = n)`. `Action` is the type of a handler with no value, and
`Action(a)` of one that receives an `a`.

## Markup

A component's `view` is markup: elements, text, other components, and the
control flow below.

```weft
view = section(class: "list") {
  h2 { list.name }
  "Plain text is a child too."
  ListStats(list: list)
  input(bind: draft, placeholder: "Add an item")
}
```

- **An element** is `name(attributes) { children }`. Either part can be left
  out: `br`, `h2 { "Title" }`, `input(bind: draft)`.
- **A component** is used the same way: `ListStats(list: list)`, or just
  `Sidebar` when it has no parameters.
- **Text** children are text values: `"…"`, or any expression with a
  `to_text` method.
- **`nothing`** renders nothing, for a branch with nothing to show.
- **Elements** are the HTML elements, with names in `snake_case`, and
  attributes are their HTML attributes, also in `snake_case`
  (`aria_label`, `tab_index`). See [Standard library: markup](stdlib#markup).

### Attributes

| Attribute | Takes | Notes |
|---|---|---|
| `on_click`, `on_submit`, … | a handler | `on_input` passes `Text`, `on_change` on a checkbox passes `Bool`, `on_key` passes a `Key` |
| `bind` | a `state` | two-way: the element shows the state and updates it as the person types or clicks |
| `class` | a `Text`, or a record of `Bool`s | `class: { done: item.done, urgent: item.urgent }` applies each class whose value is `True` |
| `key` | any value with equality | identifies an element in a loop; see [Loops](#loops) |
| everything else | its HTML type | `placeholder: "…"`, `disabled: True` |

### Conditionals

`if` and `match` work in markup, and each branch is markup:

```weft
view = div {
  if editing { input(bind: draft) } else { span { item.title } }
  match status(items_in(list.id, show)) {
    Loading      -> p { "Loading…" }
    Failed(why)  -> p(class: "error") { why }
    Ready(rows)  -> ItemList(items: rows)
    Stale(rows)  -> ItemList(items: rows)
  }
}
```

### Loops

```weft
ul {
  for i in items_in(list.id, show) { ItemRow(item: i) }
}
```

- `for` in markup repeats its body for each element.
- **Keys:** each repeated child needs an identity, so its state stays attached
  to the right item when the list changes. A record with a `key` field is
  keyed by it automatically. Anything else is keyed by its value, or by a
  `key:` attribute on the child: `li(key: row.list.id) { … }`.

## Loading and offline states

Reading a query in a view gives its last known value: what the client has
synced so far. Before the first value arrives, the component shows nothing.

To show loading, offline or failure states, wrap the query in `status`:

| `status(q)` is | When |
|---|---|
| `Loading` | no value yet |
| `Ready(value)` | in sync with the server |
| `Stale(value)` | offline, or behind; showing the last known value |
| `Failed(reason)` | the query hit a [runtime error](runtime-errors) |

## Styling

- **`style { … }`** in a component holds CSS that applies to that component
  only. A selector like `.done` can't leak into other components.
- **Global CSS** goes in `ui/styles/`. Every `.css` file there applies to the
  whole client.
- **The `class` attribute** connects the two: `class: "error"`, or
  `class: { done: item.done }`.

## Pages and routes

- **Each file in `ui/pages/` is a page,** and its path is its URL:

  | File | URL |
  |---|---|
  | `ui/pages/index.weft` | `/` |
  | `ui/pages/lists/[id].weft` | `/lists/:id` |
  | `ui/pages/lists/[id]/settings.weft` | `/lists/:id/settings` |
  | `ui/pages/404.weft` | anything no other page matches |

- **A page is a component whose parameters come from the URL.** A `[name]`
  segment becomes the parameter `name`, parsed to its declared type. Other
  parameters come from the query string and must have defaults:

  ```weft
  component ListPage(id: Id(TodoList), show: Filter = All) {
    view = match find_list(id) {
      None       -> p { "This list was deleted on another device" }
      Some(list) -> ListPanel(list: list, show: show)
    }
  }
  ```

  `/lists/01J…?show=Active` renders `ListPage(id: …, show: Active)`. If a path
  segment doesn't parse, `404.weft` is shown.

- **Layouts:** `ui/App.weft` is the client's root component and wraps every
  page. A `_layout.weft` file in a folder wraps every page in that folder. In
  either, `outlet` marks where the page goes:

  ```weft
  component App {
    view = main {
      Sidebar
      outlet
    }
  }
  ```

- **Links and navigation are typed.** A page's name with its parameters is a
  route value:
  - `link(to: ListPage(id: list.id)) { list.name }` renders a link. A link to
    the current page is marked as current.
  - `go(Home)` in a handler navigates once the handler finishes.

  A link to a page that doesn't exist, or with the wrong parameters, is a
  compile error.

- **Pages are never rendered directly.** `ListPage(id: x)` in markup is a
  compile error: pages only appear through `outlet`, so the page syntax always
  means a route.
