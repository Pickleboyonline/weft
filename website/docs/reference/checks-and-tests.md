---
title: Checks and tests
description: Invariants the simulator keeps true, expect for pure functions, and how strongly checks are verified.
---

# Checks and tests

weft has two kinds of test, and neither involves writing test scenarios by
hand.

## Checks

A **check** states something that must always be true of the data. It's a
`Check` value, holding a function that takes nothing and returns a `Bool`:

```weft
titles_not_blank = Check(|| items.all(|i| not i.title.trim().is_empty()))

/// Every item belongs to a list that exists.
no_orphans = Check(|| items.all(|i| lists.has(i.list)))
```

- **Why a value:** a function that returns a `Bool` and reads tables isn't
  necessarily a rule the app must keep (`list_is_empty` isn't), so the
  compiler can't infer which ones are. `Check(…)` says so. By convention,
  checks live in `checks/`.
- A check may only read: writing a table, sending a command, or using the
  clock or randomness in one is a compile error.
- **The simulator evaluates every check** on the server's data after every
  step of every simulated run. See [Deployment: simulations](deployment#simulations).
- **Two checks are always on,** without being written:
  1. every simulated run must match some one-at-a-time run of the same
     commands on one machine;
  2. once the network goes quiet, every query result a client is showing must
     equal what that query returns on the server: no screen is left stale.

When a check fails, the simulator reports the shortest sequence of actions it
could find that breaks it, and a seed that replays the run exactly.

## `expect`

`expect` states something about pure code, beside the code it describes:

```weft
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}

expect keeps(Active, { done: True }) == False
expect keeps(All, { done: True })
```

- `expect` takes a `Bool` expression, and it must be pure: it can't read
  tables, the clock or randomness.
- `weft test` evaluates every `expect` in the project. A failure shows the
  values of each named part of the expression.
- `expect` lines aren't part of the program: they never run in a deployment.

## How strongly a check is verified

By default, checks are verified by random simulation. A simulation can also
**exhaust** a small configuration: try every possible order of every action,
for a few clients doing a few things each.

```weft
small = Simulation {
  clients: 2,
  exhaust: { actions_per_client: 3 },
}
```

Exhausting finds every failure within its bound, which catches more than it
sounds like: most concurrency bugs need only two or three actions to show up.
The delete race (an item added to a list being deleted elsewhere) is a
two-action failure. For the trade-offs between simulation, exhaustive checking
and proof, see [Testing and proof](/design/testing-and-proof).
