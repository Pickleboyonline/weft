---
title: Bindings and variables
description: Naming values with =, mutable variables with var, scope, and destructuring.
---

# Bindings and variables

## Bindings

`name = expression` gives a value a name. The name can't be reassigned.

```weft
cleaned = title.trim()
cleaned = cleaned.lower()       # error: `cleaned` is already bound; bindings can't change
```

- **At the top level** of a file, bindings are visible throughout the project,
  in any order. A top-level binding is a constant, a function, or a resource
  such as a [`Table`](data#tables).
- **In a block,** a binding is visible from its line to the end of the block.

## Variables

`var` declares a variable inside a function body. Assign to it again with
plain `=`.

```weft
count_open = |items| {
  var open = 0
  for i in items {
    if not i.done { open = open + 1 }
  }
  open
}
```

- `var` is only allowed inside a function or handler body, never at the top
  level.
- A function nested inside can read the variable but can't assign it:

  ```weft
  tally = |items| {
    var total = 0
    add = |n| total = total + n        # error: only the function that declared `total` can assign it
    total
  }
  ```

- So a function with a `var` is still pure: the change never escapes it.

The only other things that can change are a component's
[`state`](ui#state) and [tables](data#tables).

## Scope and shadowing

A name can't be declared where the same name is already in scope, including
by an enclosing function's parameters. The compiler reports it as an error, so
each name means exactly one thing in any given place.

## Destructuring

A pattern on the left of `=` takes a value apart:

```weft
{ title, done } = item
Some(list) = lists.get(id)      # error: the pattern must cover every case; `None` isn't handled
```

A destructuring binding must cover every possible value. When it can't, use
[`match`](control-flow#match).
