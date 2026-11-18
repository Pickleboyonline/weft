---
title: Bindings and variables
description: Definitions with =, local bindings with let, variables with let mut, shadowing, and destructuring.
---

# Bindings and variables

## Definitions

At the top level, and for a record's methods and a component's members,
`name = expression` defines a name:

```weft
items = Table(Item)
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}
```

- Definitions are visible throughout their scope, in any order: top-level
  ones throughout the project, members throughout their record or component.
- A top-level definition is a constant, a function, or a value such as a
  [`Table`](data#tables). Defining the same top-level name twice is a compile
  error.

## Local bindings: `let`

Inside a block (a function body, a handler, a `match` arm), `let` binds a
name from its line to the end of the block:

```weft
add_item = Command(|list: Id(TodoList), title: Text| -> Result(Id(Item), [EmptyTitle, NoSuchList]) {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  if not lists.has(list) { return Err(NoSuchList) }
  Ok(items.insert(Item { list, title: cleaned }))
})
```

A `let` binding can't change. Writing `name = …` for it is an error:

```weft
fix_title = |title| {
  let cleaned = title.trim()
  cleaned = cleaned.lower()       // error: `cleaned` isn't mutable: write `let cleaned = …` for a new binding, or declare it with `let mut`
  cleaned
}
```

## Shadowing

A `let`, a parameter or a pattern may reuse a name that's already in scope.
The new name hides the old one until the end of its block:

```weft
normalize = |title| {
  let cleaned = title.trim()
  let cleaned = cleaned.lower()     // a new binding that shadows the first
  cleaned
}

count_open = |items| items.count_if(|i| not i.done)    // `items` here is the parameter, not the table
```

Because a new binding always starts with `let`, shadowing never looks like a
change: `let x = …` makes a new `x`, and `x = …` changes a variable or state.

## Variables: `let mut`

`let mut` declares a variable inside a function body. Assign to it with plain
`=`.

```weft
count_open = |items| {
  let mut open = 0
  for i in items {
    if not i.done { open = open + 1 }
  }
  open
}
```

- `let mut` only works inside a block, never at the top level.
- A function nested inside can read the variable but can't assign it:

  ```weft
  tally = |items| {
    let mut total = 0
    let add = |n| total = total + n        // error: only the function that declared `total` can assign it
    total
  }
  ```

- So a function with a `let mut` is still pure: the change never escapes it.

The only other things that can change are a component's
[`state`](ui#state) and [tables](data#tables). Plain `name = …` inside a block
is always an assignment, to a `let mut` or a `state`.

## Destructuring

A pattern after `let` takes a value apart:

```weft
let { title, done } = item
let Some(list) = lists.get(id)     // error: the pattern must cover every case; `Some` is only one of them. Add `else { … }` for the others, or use `match`
```

A destructuring `let` must cover every possible value, unless it says what
happens otherwise. `let … else` handles the cases the pattern doesn't match:

```weft
move_item = Command(|id: Id(Item), to: Id(TodoList)| -> Result({}, [NoSuchItem, EmptyTitle, NoSuchList]) {
  let Some(item) = items.get(id) else { return Err(NoSuchItem) }
  add_item(to, item.title)?
  items.delete(id)
  Ok({})
})
```

- The `else` block must leave: end it with `return`, `break`, `continue` or
  `crash`. The pattern's names don't exist there, so the code after the `let`
  can't run.
- The value of a `let … else` can't end with an `if` or `match`, since its
  `else` would belong to them. Put such a value in parentheses.

For several cases, use [`match`](control-flow#match); for one case you only
want to act on, use [`if let`](control-flow#if-let).
