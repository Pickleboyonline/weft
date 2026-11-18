---
title: Expressions and operators
description: Operators and precedence, calls and methods, blocks, lambdas and text.
---

# Expressions and operators

Almost everything in weft is an expression with a value, including `if`,
`match` and blocks.

## Operators

From tightest to loosest:

| Precedence | Operators | Meaning |
|---|---|---|
| 1 | `x.f`, `x.f(…)`, `f(…)`, `x?` | field, method call, call, [early return](#the--operator) |
| 2 | `-x`, `not x` | negation |
| 3 | `*`, `/`, `%` | multiply, divide, remainder |
| 4 | `+`, `-` | add, subtract |
| 5 | `..<`, `..=` | ranges: up to (excluding), up to and including |
| 6 | `==`, `!=`, `<`, `<=`, `>`, `>=` | comparison (can't be chained) |
| 7 | `and` | logical and (short-circuits) |
| 8 | `or` | logical or (short-circuits) |

- **Division:** `/` divides two numbers of the same type. On `Int` it rounds
  toward zero (`7 / 2 == 3`, `-7 / 2 == -3`), and `%` is the remainder, with
  the sign of the left side (`-7 % 2 == -1`). Dividing by zero is a
  [runtime error](runtime-errors).
- **No mixing number types:** `1 + 2.5` works because `1` becomes a `Dec`, but
  an `Int` variable plus a `Dec` needs an explicit `n.to_dec()`.
- **Comparison doesn't chain:** `a < b < c` is a compile error; write
  `a < b and b < c`.
- **Operators are methods.** `a + b` calls `a.plus(b)`, `a == b` calls
  `a.equals(b)`, `a < b` uses `a.compare(b)`. A record that defines those
  methods works with the operators. See
  [Functions: methods](functions#methods).

## Calls and methods

```weft
keeps(Active, item)                 // calling a function
title.trim()                        // calling a method of title's type
items.filter(|i| i.done).count()   // chained: a table read, then a list method
```

- `value.method(a, b)` calls `method` from the type of `value`, with `value` as
  the first argument. The compiler resolves it at compile time.
- `.field` on its own is a function: `.created` means `|x| x.created`, as in
  `sort_by(.created)`.

## Blocks

Braces around statements make a block. Its value is the value of its last
line.

```weft
label = {
  base = item.title.trim()
  if item.done { "${base} (done)" } else { base }
}
```

## Lambdas

```weft
double = |n| n * 2
same   = || 42
pair   = |a, b| { first: a, second: b }
```

A lambda can use names from where it was written (a closure). Lambdas can't
assign variables declared outside them (see [Variables](bindings#variables-let-mut)),
with one exception: inside a component, a lambda can assign that component's
[`state`](ui#state). That's what makes it a handler.

## Records, lists and tags

```weft
point   = { x: 1, y: 2 }
moved   = { ..point, x: 5 }         // copy with x changed
numbers = [1, 2, 3]
maybe   = Some(numbers)
```

## Text interpolation

`"${expr}"` inserts `expr.to_text()`. Any expression can go inside, including
method calls: `"${names.count()} names"`.

## The `?` operator

Inside a function that returns a `Result`, `x?` unwraps `Ok(v)` to `v`, and
returns `Err(e)` from the function straight away:

```weft
parse_pair : Text -> Result({ a: Int, b: Int }, [Missing, NotANumber])
parse_pair = |text| {
  let parts = text.split(",")
  let a = parts.get(0).ok_or(Missing)?.to_int()?     // to_int fails with NotANumber
  let b = parts.get(1).ok_or(Missing)?.to_int()?
  Ok({ a, b })
}
```

The error reasons from every `?` combine into the function's error type, the
same way tag unions grow anywhere else.

`?` is only for `Result`. It works the same in commands, where returning
`Err` also undoes the command's writes (see
[Commands: failing](commands-and-queries#failing)).
