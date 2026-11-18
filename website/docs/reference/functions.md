---
title: Functions and methods
description: Defining functions, signatures, purity, methods on records, well-known methods and where clauses.
---

# Functions and methods

## Defining functions

A function is a binding whose value is a lambda:

```weft
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}
```

A **signature** on the line before gives its type. It's optional except where
[Types: annotations](types#annotations) requires it:

```weft
keeps : Filter, Item -> Bool
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}
```

Top-level functions can be used anywhere in the project, before or after
their definition.

## What a function is

There's no keyword for "command" or "query". A function's role follows from
two things the compiler already knows: what it touches (its
[effects](effects)) and which folder it's in.

| A function that… | in… | is a… | which means |
|---|---|---|---|
| writes tables | `commands/` | **command** | clients can call it; it runs as a transaction. See [Commands](commands-and-queries#commands). |
| only reads tables | `queries/` | **query** | views can read it, and stay in sync. See [Queries](commands-and-queries#queries). |
| is `Bool`, reads tables | `checks/` | **check** | the simulator keeps it true. See [Checks](checks-and-tests). |
| changes component state | a `component` | **handler** | it runs in response to an event. See [UI](ui#handlers). |
| anything else | anywhere | **helper** | an ordinary function, callable from the others |

A function in `commands/` that doesn't write anything, or one in `queries/`
that writes, is a compile error.

## Purity

- A function without `!` in its name is **pure**: same arguments, same result,
  and no effect on anything outside it. It may read tables (those reads are
  tracked: see [Effects](effects#reactive-reads)).
- A function whose name ends in `!` may change the world. Only `!` functions
  can call `!` functions, with one exception: component handlers, where a call
  to a `!` function is *sent* rather than run (see [UI](ui#calling-commands)).

## Methods

A record's methods are bindings inside its declaration. A method's first
parameter is the value it's called on.

```weft
record Money {
  cents:    Int
  currency: Currency

  plus    = |a, b| Money { cents: a.cents + b.cents, currency: a.currency }
  to_text = |m| "${m.cents // 100}.${m.cents % 100} ${m.currency.to_text()}"
  zero    = |currency| Money { cents: 0, currency }
}
```

- `price.plus(tax)` calls `Money.plus(price, tax)`: the compiler finds `plus`
  on the type of `price`.
- `Money.zero(Usd)` calls a method directly through its type. Use this for
  methods that don't take a `Money` first.
- Methods are resolved at compile time. A call to a method the type doesn't
  have is a compile error.

## Well-known methods

Some method names are used by syntax. Defining them makes your type work with
it:

| Method | Used by |
|---|---|
| `equals : a, a -> Bool` | `==`, `!=` |
| `compare : a, a -> [Less, Equal, Greater]` | `<`, `<=`, `>`, `>=`, `sort` |
| `plus`, `minus`, `times`, `divided_by` | `+`, `-`, `*`, `/` |
| `negate` | unary `-` |
| `to_text : a -> Text` | `"${…}"`, and display in the UI |
| `hash` | `Dict` keys and `Set` members |
| `iter` | `for … in` |

`equals`, `compare`, `to_text` and `hash` are derived automatically for every
record and tag union (see [Types](types#equality-ordering-and-text)). Defining
one replaces the derived version.

## `where` clauses

A generic function says which methods its type variables need:

```weft
largest : List(a) -> Option(a) where [a.compare : a, a -> [Less, Equal, Greater]]
largest = |values| values.fold(None, |best, v| match best {
  None    -> Some(v)
  Some(b) -> if v > b { Some(v) } else { best }
})
```

Any type with a matching `compare` can be passed. There are no interfaces to
declare or implement. Without an annotation, the compiler infers the `where`
clause.
