---
title: Checks and tests
description: Invariants the simulator keeps true, expect for pure functions, and how strongly checks are verified.
---

# Checks and tests

weft has two kinds of test, and neither involves writing test scenarios by
hand.

## Checks

A **check** is a top-level `Bool` binding in `checks/`. It states something
that must always be true of the data:

```weft
titles_not_blank = items.all(|i| not i.title.trim().is_empty())

## Every item belongs to a list that exists.
no_orphans = items.all(|i| lists.has(i.list))
```

- A check may read tables and call pure functions. It can't write anything.
- **The simulator evaluates every check** on the server's data after every
  step of every simulated run, and on each client's local copy once its sync
  settles. See [Deployment: simulations](deployment#simulations).
- **Two checks are always on,** without being written:
  1. every simulated run must match some one-at-a-time run of the same
     commands on one machine;
  2. once the network goes quiet, every client's query results must equal
     the server's.

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

- `expect` takes a `Bool` expression. It can't read tables or call `!`
  functions.
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
