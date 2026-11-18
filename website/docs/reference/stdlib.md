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
| `push(x)` | in a handler, on a `state` or `var`: adds `x` in place |
| `zip(list)` | `List({ first: a, second: b })` |
| `iter()` | used by `for` |

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
| `is_ok()`, `is_err()` | `Bool` |

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
| `new_id!()` | a new `Id(t)` |
| `id.to_text()` | `Text`: the 26-character URL-safe form |
| `Id.parse(text)` | `Result(Id(t), [NotAnId])` |

## Tables

See [Data: tables](data#reading) for the full table API.

## Built-in effectful functions

`now!()`, `today!()`, `new_id!()` and `random!(range)`: see
[Effects](effects#built-in-effectful-functions).

## UI

| Value or function | Meaning |
|---|---|
| `nothing` | renders nothing |
| `outlet` | where a layout renders the current page |
| `go(route)` | in a handler: navigate once the handler finishes |
| `status(query)` | `Loading`, `Ready(v)`, `Stale(v)` or `Failed(reason)`: see [UI](ui#loading-and-offline-states) |
| `Action`, `Action(a)` | the types of handlers passed to components |

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

Attributes are the HTML attributes in `snake_case`, plus `bind`, `key`, and
`class` as a record of `Bool`s: see [UI: attributes](ui#attributes).

## Configuration types

| Type | Used in | See |
|---|---|---|
| `Table(r, …)` | `data/` | [Data](data#tables) |
| `Extern { needs, sim }` | `externs/` | [Externs](resources#externs) |
| `Job { every \| at, run }` | `jobs/` | [Jobs](resources#jobs) |
| `Endpoint { method, path, run }` | `endpoints/` | [Endpoints](resources#endpoints) |
| `Deployment { … }`, `Machine { … }`, `Budget { … }` | `deploy/` | [Deployment](deployment) |
| `Simulation { … }`, `Network { … }`, `Crashes { … }` | `simulate/` | [Simulations](deployment#simulations) |
