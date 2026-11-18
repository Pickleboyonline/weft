---
title: Control flow
description: if and else, match and patterns, loops, early return and recursion.
---

# Control flow

## `if` and `else`

```weft
label = if item.done { "done" } else { "open" }

if cleaned.is_empty() { reject EmptyTitle }

kind = if n < 0 { Negative } else if n == 0 { Zero } else { Positive }
```

- The condition must be a `Bool`. There's no "truthiness".
- Braces are always required, even for a single value.
- As an expression, `if` needs an `else` and both branches must have the same
  type. As a statement (its value unused), `else` is optional.

```weft
label = if item.done "done" else "open"   # error: `if` branches need braces
```

## `match`

```weft
describe = |shape| match shape {
  Circle(r)            -> "circle of radius ${r}"
  Rect({ w, h }) if w == h -> "square of side ${w}"
  Rect({ w, h })       -> "${w} by ${h} rectangle"
}
```

- **Arms** are `pattern -> expression`, checked in order.
- **Exhaustive:** the arms must cover every possible value. A missing case is a
  compile error, and the error names it.
- **Guards:** `if condition` after a pattern adds a test.
- **Alternatives:** `A | B -> …` matches either pattern.
- **`_`** matches anything and binds nothing.

### Patterns

| Pattern | Matches |
|---|---|
| `x` | anything, and names it `x` |
| `_` | anything |
| `42`, `"text"`, `True` | that exact value |
| `Some(x)`, `Rect({ w, h })` | that tag, taking its payload apart |
| `{ title, done }` | a record, naming those fields |
| `[]`, `[first, ..rest]` | an empty list; a list's first element and the rest |
| `A \| B` | either |

Patterns also work in `=` bindings and `for` loops, where they must cover
every case.

## Loops

```weft
open_count = |items| {
  var n = 0
  for i in items {
    if i.done { continue }
    n = n + 1
  }
  n
}

first_overdue = |items, today| {
  for i in items {
    if i.due < today { return Some(i) }
  }
  None
}

backoff = |attempts| {
  var wait = 1s
  var tries = 0
  while tries < attempts {
    wait = wait * 2
    tries = tries + 1
  }
  wait
}
```

- **`for pattern in collection { … }`** works on anything with an `iter`
  method: lists, sets, dicts (as `{ key, value }` records), ranges, and tables.
- **`while condition { … }`** repeats while the condition holds.
- **`break`** leaves the innermost loop; **`continue`** skips to its next
  round.
- A loop is a statement. Its body produces no value; loops change `var`s or
  call functions.
- Looping over a table inside a function reads the table, which shows up in
  the function's [effects](effects).
- In markup, `for` produces markup instead: see [UI: markup](ui#loops).

## Early return

`return value` leaves the enclosing function with that value. Inside a lambda,
it leaves the lambda. The `?` operator is a short form for returning errors:
see [Expressions](expressions#the--operator).

## Recursion

Functions can call themselves and each other. Tail calls (a call that is the
last thing a function does) don't grow the stack.

```weft
depth = |node| match node.parent {
  None    -> 0
  Some(p) -> 1 + depth(p)
}
```
