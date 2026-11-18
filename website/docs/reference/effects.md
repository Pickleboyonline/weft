---
title: Effects
description: The ! rule, effect rows, built-in effect kinds, and user-defined effects and handlers.
---

# Effects

An *effect* is anything a function does besides compute its result from its
arguments: writing a table, reading the clock, sending a command. weft tracks
effects in types, so the compiler always knows exactly what each function
touches.

## The rule

A function's name ends in `!` exactly when its effects include something that
changes the world or isn't deterministic:

| Effect kind | Written in a row as | Needs `!`? |
|---|---|---|
| Writing a table | `write items` | yes |
| Rejecting (commands only) | `reject [EmptyTitle]` | yes |
| Reading the clock | `time` | yes |
| Randomness, including new ids | `random` | yes |
| Calling an extern | `extern send_email!` | yes |
| A user-defined effect | `Audit` | yes |
| Reading a table | `read items` | no: [reactive reads](#reactive-reads) |
| Setting component state | `set draft` | no: see [UI](ui#state) |
| Sending a command from a component | `send add_item!` | no: see [UI](ui#calling-commands) |

The compiler checks both directions: a function with `!` effects must have a
`!` name, and a `!` name on a pure function is an error too.

```weft
stamp = |item| { ..item, seen: now!() }    # error: `stamp` reads the clock, so it must be named `stamp!`
```

## Effect rows

A function's type lists its effects after `!`, between `<` and `>`:

```weft
add_item!  : Id(TodoList), Text => Id(Item) ! <write items, read lists, reject [EmptyTitle, NoSuchList], random, time>
items_in   : Id(TodoList), Filter -> List(Item) ! <read items>
keeps      : Filter, Item -> Bool
```

- Rows are inferred. Your editor shows them on hover, and `weft explain`
  prints them.
- In a signature you write, a row may end with `..`, meaning "and whatever
  else the body does". Commands do this to declare their rejections without
  listing every table: `! <reject [EmptyTitle, NoSuchList], ..>`.
- Rows combine: a function has the effects of everything it calls.

## Reactive reads

Reading a table is tracked but doesn't need `!`, because it neither changes
anything nor varies by itself. For a given set of rows, a read gives the same
answer every time. When the rows change, anything that read them is
recomputed. That's how [queries](commands-and-queries#queries) stay in sync on
every client.

## Built-in effectful functions

| Function | Effect | Returns |
|---|---|---|
| `now!()` | `time` | the current `Time` |
| `today!()` | `time` | the current `Date` |
| `new_id!()` | `random` | a fresh `Id(t)`, whose type comes from context |
| `random!(range)` | `random` | a random number in the range |

Every platform provides these. In a simulation, they come from the run's seed,
so a run replays exactly.

## User-defined effects

You can declare your own effect: a set of operations that a *handler*
provides. The code using the effect doesn't know or care which handler is
installed.

```weft
effect Audit {
  note! : Text => {}
}

clear_done! : Id(TodoList) => {} ! <..>
clear_done! = |list| {
  items.delete_where!(|i| i.list == list and i.done)
  Audit.note!("cleared done items in ${list}")
}

handler audit_to_table : Audit {
  note! = |event| audit_log.insert!(AuditEntry { event })
}
```

- Inside an `effect`, each operation is a signature, and its name ends in `!`
  like any effectful function. Calling one (`Audit.note!(…)`) adds the effect
  to the caller's row.
- A `handler` gives each operation an implementation.
- **Installing a handler:** `with audit_to_table { … }` installs it for the code
  in the braces. A deployment can install handlers for every command with its
  `handlers` field (see [Deployment](deployment#deployments)).

### Operation kinds

An operation's kind says how its handler may return to the caller, and so
where the handler may run:

| Kind | Declared as | The handler… | May the handler run on another machine? |
|---|---|---|---|
| resume-once (default) | `note! : Text => {}` | returns a value, exactly once | yes: it becomes a request and a response |
| final | `final abort! : Text => a` | never returns; the caller stops | yes: it becomes an abort with a reason |
| control | `ctl choose! : List(a) => a` | may return any number of times | no: it must run on the caller's machine |

`reject` is a built-in final operation. Control operations are for things like
backtracking search and generators. A control operation captures the rest of
the computation, so it can't travel over the network. The compiler rejects any placement that would put a
control handler on a different machine from its caller.
