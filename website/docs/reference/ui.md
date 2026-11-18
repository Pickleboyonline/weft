---
title: UI
description: Components, state, handlers, markup, styling, pages and routing.
---

# UI

A weft client is a tree of **components**. Each one has its own state, the
functions that change it, its markup, and its styles. A few general
constructs cover everything else a client needs: waiting with `.await`, work
tied to a component's life with `task`, handles to elements on screen,
reusable stateful functions, and platform modules for the outside world. Data
from another service, a live socket, or a canvas widget is ordinary weft code
built from these, not a special case.

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
    rename_item(item.id, draft).await.on_err(|err| problem = err)
  }

  view = li {
    checkbox(checked: item.done, on_change: |d| set_done(item.id, d))
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
  bindings, handlers, [tasks](#tasks), one `view`, and an optional `style`.

## State

`state name = initial` declares a value that belongs to one instance of the
component and can change.

- **Its type is inferred.** `state problem = None` above gets the type
  `[None, EmptyTitle]` from the assignment `problem = err`, because
  `rename_item` can fail with `EmptyTitle`. Annotate it when inference can't
  tell: `state selected: Option(Id(Item)) = None`.
- **Only this component's functions can assign it,** with plain `=`. Assigning
  it in `view`, or in a binding that isn't a function, is a compile error.
- **State must be data:** records, tags, lists, and opaque handles such as an
  [`Element`](#element-handles) or a socket. It can't hold functions. So it
  can always be logged, replayed and kept across a reload.
- **Each instance has its own.** Two `ItemRow`s have separate `editing` states.
  In a loop, the key decides which state belongs to which row
  ([loops](#loops)).

## Derived values

Any ordinary binding in a component that reads state, parameters or queries
is recomputed whenever they change. These bindings, and `view`, may read
tables and queries but not change anything (see
[Effects](effects#where-effects-are-allowed)):

```weft
remaining = items_in(list.id, All).count_if(|i| not i.done)
label = if remaining == 1 { "1 item left" } else { "${remaining} items left" }
```

Nothing has to be declared as "derived".

## Handlers

A **handler** is a function in a component that assigns that component's
state. It runs in response to an event.

- Named: `edit = || { … }`, then `on_click: edit`.
- Inline: `on_click: || editing = False`.
- With an event value: `on_change: |d| set_done(item.id, d)` receives the
  checkbox's new `Bool`.

Handlers and [tasks](#tasks) are the only way state changes. A handler runs
without interruption until it finishes or reaches an
[`.await`](#waiting-await), so it never sees half of another handler's
changes. While it waits, other handlers can run.

What it compiles to: every handler, named or inline, becomes a message, and
all of a component's assignments become one pure update function: the Elm
Architecture. That's what lets the simulator replay any run.

### Calling commands

Inside a handler, calling a command sends it. Adding `.await` waits for the
server's result. See
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

## Waiting: `.await`

Some calls take time: a command's result from the server, a request to
another service, a timer, the next message on a socket. Calling one starts it.
`.await` after the call waits for its value:

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

- **Without `.await`, a call that takes time gives a `Pending(a)`,** which can
  be awaited later. That's how to start two things at once:
  `a = http.get(x)`, `b = http.get(y)`, then `a.await` and `b.await`.
- **A function that uses `.await` takes time too,** so calling it needs
  `.await` to get its value.
- **`.await` is the one place where other code can run in between.** State
  read before it may have changed after it, and later reads see a newer
  [snapshot](effects#snapshots). That's why it's written, while effects
  aren't: it's control flow, like `return`.
- **A `Result` can't be ignored.** Calling a command that can fail without
  waiting for it and handling its result is a compile error. `.on_err(f)` is
  the short form when only the failure matters:
  `rename_item(id, draft).await.on_err(|err| problem = err)`.
- **`.await` works in handlers, tasks, stateful functions, jobs and
  endpoints.** Commands, queries, views and checks can't wait: they run
  atomically, or as functions of one snapshot.
- If the component goes away while a handler is waiting, the handler is
  cancelled. A command it sent still runs on the server.
- In a simulation, the simulator decides when each awaited call finishes, so
  it explores the orders in which handlers can interleave.

## Tasks

A **task** is work tied to a component's life. It starts when the component
appears and is cancelled when the component goes away. Use one for anything
that runs over time: a timer, a live connection, loading data from another
service.

```weft
component Prices(symbol: Text) {
  state latest = None

  task(key: symbol) {
    let socket = ws.connect("wss://prices.example.com/${symbol}").await
    defer { socket.close() }
    while True {
      match socket.receive().await {
        Ok(msg) -> msg.decode(Quote).on_ok(|q| latest = Some(q))
        Err(_)  -> break
      }
    }
  }

  view = match latest {
    None    -> p { "Connecting…" }
    Some(q) -> p { "${symbol}: ${q.price}" }
  }
}
```

- **`task { … }`** runs once each time the component appears.
  **`task(key: value) { … }`** also restarts whenever `value` changes: the
  running task is cancelled first. A task never restarts by itself because of
  state it reads or assigns.
- **Cancellation** happens at an `.await`. `defer { … }` runs its block when
  the enclosing block ends, whether normally, by an error, or by
  cancellation (see [Control flow](control-flow#defer)). There's nothing to
  unsubscribe by hand.
- A component can have several tasks. Like handlers, a task assigns its
  component's state, and runs uninterrupted between `.await`s.

## Element handles

`ref: name` on an element puts a handle to it in a state of type
`Option(Element)` once it's on screen, for focusing, measuring, scrolling or
drawing:

```weft
component Search {
  state query = ""
  state box: Option(Element) = None

  task { box.map(|b| b.focus()) }

  view = input(ref: box, bind: query, placeholder: "Search")
}
```

An `Element` is an opaque handle, like an `Id`: data that can be compared and
stored in state. What it can do (focus, size, scroll, a 2D or WebGL drawing
context) comes from the platform's `dom` module (see
[Standard library](stdlib#platform-modules)). In a simulation it's a stand-in
that records what was done to it.

## Stateful functions

A function whose body declares `state` or `task` is a **stateful function**:
reusable logic with its own state, like a custom hook in React. The state
belongs to the component that calls it, and each call has its own.

```weft
/// Load `key` with `load`, and reload it every `fresh_for`.
remote = |key, load, fresh_for| {
  state result = Loading
  task(key: key) {
    while True {
      result = match load(key).await {
        Ok(v)  -> Ready(v)
        Err(e) -> Failed(e)
      }
      sleep(fresh_for).await
    }
  }
  result
}

component Weather(city: Text) {
  forecast = remote(city, |c| http.get("https://api.example.com/w/${c}").await?.decode(Forecast), 10min)

  view = match forecast {
    Loading   -> p { "…" }
    Ready(f)  -> p { f.summary }
    Failed(_) -> p { "Forecast unavailable" }
  }
}
```

- A stateful function is called at the top level of a component's body, or of
  another stateful function: never inside an `if`, a loop or a lambda. That
  way each call keeps the same state from one render to the next. Calling one
  anywhere else is a compile error.
- Libraries are built this way. A caching layer with a cache shared across
  components (in a [`Memory` table](data#options)), deduplicated requests,
  refresh on focus and reconnect, and paging is a stateful function and a
  table, written in weft.

## Data from outside the app

Tables and queries cover the app's own data. For everything else, each
platform provides modules of ordinary functions: `http`, `ws`, `sleep`, `dom`
and `browser` on the client, and `http`, `sleep` and `secrets` on the server
(see [Standard library: platform modules](stdlib#platform-modules)). Their
effects are inferred like any other.

- **Data from other services isn't part of the app's state.** The simulator
  treats each response as input from outside, drawn from the simulation's
  [models](deployment#simulations), not as something the one-machine run must
  reproduce.
- **Commands can't call the outside world.** A command is atomic and may run
  twice, so a request in the middle of one can't be undone or repeated
  safely. Call the service from a handler or task and pass the result to a
  command, or use a [job](resources#jobs) that watches a query.
- **Existing host libraries,** such as a JavaScript charting library, can be
  used through an [extern](resources#externs). That's rarely needed: the
  platform modules cover the outside world, and the constructs above cover
  the rest.

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
| `ref` | a `state` of type `Option(Element)` | set to a handle to the element while it's on screen; see [Element handles](#element-handles) |
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
    Refreshing(rows) -> ItemList(items: rows)
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

## Loading states

Calling a query in a view gives its latest result (see
[Using a query in a component](commands-and-queries#using-a-query-in-a-component)).
A component isn't shown until every query it calls has a first result, so a
result never means "not loaded yet". After that, while a refresh is on its
way, the previous result stays on screen.

To show loading, refreshing or failure states, wrap the call in `status`. A
query read through `status` doesn't hold the component back: it reports
`Loading` instead.

| `status(q)` is | When |
|---|---|
| `Loading` | no result yet |
| `Ready(value)` | the latest result, with no refresh in flight |
| `Refreshing(value)` | showing the previous result while a new one is on its way |
| `Failed(reason)` | the query failed: a [runtime error](runtime-errors), or the server couldn't be reached |

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
      Some(list) -> ListPanel(list: list, start: show)
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
  - `go(Home)` in a handler or task navigates once it finishes or reaches an
    `.await`.

  A link to a page that doesn't exist, or with the wrong parameters, is a
  compile error.

- **Pages are never rendered directly.** `ListPage(id: x)` in markup is a
  compile error: pages only appear through `outlet`, so the page syntax always
  means a route.
