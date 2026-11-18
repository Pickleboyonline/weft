---
title: Standard library
description: The methods of every built-in type, the markup elements, and the built-in values and functions.
---

# Standard library

Everything here is available in every file, without imports. Methods are
called with `.`: `title.trim()`.

## Numbers

`Int`, `Dec` and `Float` share these, each returning its own type unless
noted.

| Method | Returns |
|---|---|
| `abs()`, `negate()` | the absolute value; the negation |
| `min(other)`, `max(other)` | the smaller or larger |
| `clamp(low, high)` | the value limited to the range |
| `round()`, `floor()`, `ceil()` | `Int` (on `Dec` and `Float`) |
| `round_to(places)` | the value rounded to `places` decimal places (on `Dec` and `Float`) |
| `to_int()`, `to_dec()`, `to_float()` | the value converted; `to_int` truncates |
| `to_text()` | the value as text |
| `is_zero()`, `is_positive()`, `is_negative()` | `Bool` |

`Text.to_int()`, `to_dec()` and `to_float()` parse text, returning
`Result(number, [NotANumber])`.

## Text

| Method | Returns |
|---|---|
| `length()` | `Int`: characters |
| `is_empty()` | `Bool` |
| `trim()`, `trim_start()`, `trim_end()` | `Text` |
| `lower()`, `upper()` | `Text` |
| `contains(t)`, `starts_with(t)`, `ends_with(t)` | `Bool` |
| `split(sep)` | `List(Text)` |
| `replace(old, new)` | `Text` |
| `slice(range)` | `Text`: the characters in the range |
| `concat(t)` | `Text` |
| `to_bytes()` | `Bytes`, as UTF-8 |
| `to_int()`, `to_dec()`, `to_float()` | `Result(number, [NotANumber])` |

## Bool

`not b`, `a and b`, `a or b`, and `to_text()`.

## List

| Method | Returns |
|---|---|
| `count()`, `count_if(pred)`, `is_empty()` | `Int`; `Int`; `Bool` |
| `get(index)`, `first()`, `last()` | `Option(a)` |
| `find(pred)` | `Option(a)`: the first match |
| `contains(x)` | `Bool` |
| `any(pred)`, `all(pred)` | `Bool` |
| `filter(pred)`, `map(f)`, `flat_map(f)` | `List` |
| `fold(start, f)` | the result of combining every element, left to right |
| `sum()`, `min()`, `max()` | the element type (`min` and `max` return `Option`) |
| `sort()`, `sort_by(f)`, `reverse()` | `List` |
| `distinct()`, `distinct_by(f)` | `List` |
| `group_by(f)` | `List({ key: k, items: List(a) })` |
| `take(n)`, `skip(n)` | `List` |
| `append(x)`, `prepend(x)`, `concat(list)` | `List` |
| `zip(list)` | `List({ first: a, second: b })` |
| `iter()` | used by `for` |

- Methods that take a function have that function's effects: `map` with a
  function that reads a table reads it too (see
  [Effects](effects#effects-follow-their-callers-and-callbacks)).
- To add to a `state` or `var`, assign a new list: `names = names.append(x)`.

## Dict and Set

| `Dict(k, v)` method | Returns |
|---|---|
| `get(k)` | `Option(v)` |
| `has(k)` | `Bool` |
| `insert(k, v)`, `remove(k)` | `Dict` |
| `keys()`, `values()` | `List` |
| `count()`, `is_empty()` | `Int`, `Bool` |

| `Set(a)` method | Returns |
|---|---|
| `has(x)` | `Bool` |
| `insert(x)`, `remove(x)` | `Set` |
| `union(s)`, `intersection(s)`, `difference(s)` | `Set` |
| `count()`, `is_empty()`, `to_list()` | `Int`, `Bool`, `List` |

`Dict.from_list(pairs)` and `Set.from_list(list)` build them; `{}` is not an
empty dict (it's the empty record): use `Dict.empty()`.

## Option and Result

| `Option(a)` method | Returns |
|---|---|
| `with_default(x)` | the value, or `x` |
| `map(f)` | `Option(b)` |
| `ok_or(err)` | `Result(a, err)` |
| `is_some()`, `is_none()` | `Bool` |

| `Result(ok, err)` method | Returns |
|---|---|
| `with_default(x)` | the value, or `x` |
| `map(f)`, `map_err(f)` | `Result` |
| `on_ok(f)`, `on_err(f)` | `{}`: calls `f` with the value or the error, if there is one |
| `is_ok()`, `is_err()` | `Bool` |

A `Result` can't be ignored: a call whose `Result` is never used is a compile
error. `.on_err(|_| {})` drops a failure on purpose, visibly.

## Pending

A call that takes time, made without [`.await`](ui#waiting-await), gives a
`Pending(a)`. `.await` on it waits for the `a`. Starting several and then
awaiting each runs them at the same time.

## Time, Date and Duration

| Expression | Returns |
|---|---|
| `time + duration`, `time - duration` | `Time` |
| `time - time` | `Duration` |
| `time.date()` | `Date` (in UTC) |
| `time.in_zone(zone)` | a local date and time, for display |
| `date.day()`, `date.month()`, `date.year()`, `date.weekday()` | `Int`, `Int`, `Int`, `Monday`…`Sunday` |
| `date + duration`, `date - duration` | `Date` (whole days: `date + 3d`) |
| `duration.seconds()`, `duration.minutes()`, `duration.hours()` | `Dec` |
| `duration * n`, `duration / n` | `Duration` |

## Id

| Expression | Returns |
|---|---|
| `new_id()` | a new `Id(t)` |
| `id.to_text()` | `Text`: the 26-character URL-safe form |
| `Id.parse(text)` | `Result(Id(t), [NotAnId])` |

## Tables

See [Data: reading](data#reading) for the full table API.

## Query methods

These work inside [queries on tables](data#queries-on-tables), and compile to
the database. Everything else in this library runs only in memory.

| Type | Query methods |
|---|---|
| All types | `==`, `!=`, `<`, `<=`, `>`, `>=` |
| Numbers | `+ - * / %`, `abs`, `min`, `max`, `round`, `floor`, `ceil`, `to_int`, `to_dec`, `to_float` |
| `Bool` | `and`, `or`, `not` |
| `Text` | `length`, `is_empty`, `trim`, `trim_start`, `trim_end`, `lower`, `upper`, `contains`, `starts_with`, `ends_with`, `concat`, `slice` |
| `Time`, `Date`, `Duration` | arithmetic and comparison, `date`, `day`, `month`, `year` |
| `Option` | `with_default`, `is_some`, `is_none`, `map` |
| `List` (results of reads, and lists passed in) | `count`, `count_if`, `is_empty`, `first`, `find`, `contains`, `any`, `all`, `filter`, `map`, `flat_map`, `sum`, `min`, `max`, `sort_by`, `reverse`, `take`, `skip`, `distinct`, `distinct_by`, `group_by` |

## Built-in effectful functions

`now()`, `today()`, `new_id()`, `random(range)`, `sleep(duration)`,
`go(route)` and `status(query)`: see
[Effects](effects#built-in-effectful-functions).

## Platform modules

Each platform provides modules for the world outside the app. They're
ordinary functions; calls that take time give a `Pending` and are used with
`.await`. In a simulation, the sim platform provides them, answering calls to
other services from the simulation's [models](deployment#simulations).

| Module | Platform | Main functions |
|---|---|---|
| `http` | client, server | `get(url)`, `post(url, body)`, `put`, `delete`, with named options `headers:`, `timeout:`, `idempotency_key:`; each gives a `Response`, then `.decode(Type)` (a `Result(Type, [BadData(Text)])`, since the data may not fit), `.text()`, `.status` |
| `ws` | client, server | `connect(url)` gives a socket with `send(message)`, `receive()` (a `Result` with the next message, or `Closed`) and `close()`; a message has `.decode(Type)` and `.text()` like a `Response` |
| `dom` | client | `Element` methods: `focus()`, `blur()`, `size()`, `scroll_into_view()`, `canvas_2d()`, `webgl()` |
| `browser` | client | `location()`, `online()`, `focused()`, `clipboard`, `geolocation()`, `notify(text)`; `online()` and `focused()` are read reactively, like queries |
| `secrets` | server | `get(name)`: a `Secret`, which can be passed to `http` but never shown, logged or sent to a client |

A socket and an `Element` are opaque handles: data, so they can be kept in
`state`, but their operations are platform calls.

## UI

| Value or function | Meaning |
|---|---|
| `nothing` | renders nothing |
| `outlet` | where a layout renders the current page |
| `go(route)` | in a handler or task: navigate once it finishes or reaches an `.await` |
| `status(query)` | `Loading`, `Ready(v)`, `Refreshing(v)` or `Failed(reason)`: see [UI](ui#loading-states) |
| `Action`, `Action(a)` | the types of handlers passed to components: `() -> {}` and `a -> {}` |
| `Element` | a handle to an element on screen, set by `ref:`; see [UI](ui#element-handles) |

## Markup

Elements are the HTML elements, written in `snake_case`, plus a few weft
additions:

| Element | Notes |
|---|---|
| `main`, `nav`, `section`, `header`, `footer`, `article`, `aside`, `div` | layout |
| `h1` … `h6`, `p`, `span`, `strong`, `em`, `code`, `pre`, `br`, `hr` | text |
| `ul`, `ol`, `li` | lists |
| `form`, `input`, `textarea`, `select`, `option`, `label`, `button` | forms |
| `img`, `a` | media and plain links |
| `checkbox` | weft addition: `input` of type checkbox, with `checked` and `on_change` passing `Bool` |
| `link` | weft addition: a typed link to a page, `link(to: Route)` |

Attributes are the HTML attributes in `snake_case`, plus `bind`, `ref`, `key`, and
`class` as a record of `Bool`s: see [UI: attributes](ui#attributes).

## Configuration types

| Type | Used in | See |
|---|---|---|
| `Table(r, …)` | `data/` | [Data](data#tables) |
| `Query(f)`, `Command(f)` | `queries/`, `commands/` | [Commands and queries](commands-and-queries) |
| `Check(f)` | `checks/` | [Checks](checks-and-tests) |
| `Extern { needs, sim }` | `externs/` | [Externs](resources#externs) |
| `Job { every \| at \| watch, run }` | `jobs/` | [Jobs](resources#jobs) |
| `Endpoint { method, path, run }` | `endpoints/` | [Endpoints](resources#endpoints) |
| `Deployment { … }`, `Machine { … }` | `deploy/` | [Deployment](deployment) |
| `Simulation { … }`, `Network { … }`, `Crashes { … }` | `simulate/` | [Simulations](deployment#simulations) |
