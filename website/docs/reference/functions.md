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
[Types: annotations](types#annotations) requires it. A lambda can also
annotate its parameters and result inline, as commands and queries do:
`|list: Id(TodoList), title: Text| -> Result(…) { … }`.

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

There's no keyword for "command" or "query". Most functions are just
functions; the few with a special role are wrapped in a standard-library
value that says so, the way a table is a `Table(…)`.

| A function… | is a… | which means |
|---|---|---|
| wrapped in `Command(…)` | **command** | a remote function the client calls to change data: atomic, and part of the API. See [Commands](commands-and-queries#commands). |
| wrapped in `Query(…)` | **query** | a remote function the client calls to read data. See [Queries](commands-and-queries#queries). |
| wrapped in `Check(…)` | **check** | an invariant the simulator keeps true. See [Checks](checks-and-tests). |
| in a component, that assigns its state | **handler** | it runs in response to an event. See [UI](ui#handlers). |
| that declares `state` or `task` | **stateful function** | reusable logic whose state belongs to the component calling it. See [UI](ui#stateful-functions). |
| anything else | **helper** | an ordinary function, callable from the others |

- Commands and queries are explicit because they're the boundary between
  client and server: the app's API, and where security and compatibility
  matter. `weft explain api` lists them.
- Where every other function runs (client, server, or both) follows from
  what it references (see
  [Server-only code](commands-and-queries#server-only-code)).
- Folders are a convention for finding things (see
  [Projects](projects#layout)). Moving a function to another folder never
  changes what it does.

## Purity

- A function with no effects is **pure**: same arguments, same result, and no
  effect on anything outside it.
- Nothing in a function's name or type says whether it's pure. The compiler
  infers it, your editor shows it, and the compiler reports an error where an
  effect isn't allowed (see [Effects](effects#where-effects-are-allowed)).

## Methods

A record's methods are bindings inside its declaration. A method's first
parameter is the value it's called on.

```weft
record Money {
  cents:    Int
  currency: Currency

  plus    = |a, b| Money { cents: a.cents + b.cents, currency: a.currency }
  to_text = |m| "${m.cents / 100}.${m.cents % 100} ${m.currency.to_text()}"
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
