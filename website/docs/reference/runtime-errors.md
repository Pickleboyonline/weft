---
title: Runtime errors
description: What happens when a program fails at run time, and crash.
---

# Runtime errors

A runtime error is a bug: something the code didn't handle and the type
checker couldn't rule out. It's different from a `reject`, which is an
expected outcome a command reports to its caller.

## What causes them

| Cause | Example |
|---|---|
| Integer overflow | an `Int` result beyond 64 bits |
| Division by zero | `n // 0`, `x / 0.0` |
| An impossible state, declared with `crash` | see [`crash`](#crash) |
| A loop or recursion that doesn't finish within its time limit | a `while` whose condition never becomes false |

Out-of-range indexing isn't one of them: `list.get(i)` returns an `Option`.

## What happens

| Where | The effect |
|---|---|
| A command | Everything it did is undone, as with `reject`. The client undoes its optimistic change and shows a general failure; the `else` handler doesn't run, because the error isn't one of the command's reasons. The error is reported to the developer. |
| A query | `status(query)` becomes `Failed(reason)`, and views reading it directly keep their last value. |
| A handler | Its state changes are discarded, and nothing it sent is sent. |
| A job or endpoint | Its transaction is undone, and the error is reported. An endpoint responds with status 500. |
| A simulation | The run fails, with its seed. Any runtime error in any run is a failure, even if every check passes. |

## `crash`

`crash` stops the current command, query, handler or job with a runtime
error. Use it for states that are impossible if the rest of the code is
right:

```weft
weekday_name = |n| match n {
  1 -> "Monday"
  2 -> "Tuesday"
  _ -> crash "weekday_name called with ${n}"
}
```

`crash` has every type, so it fits in any branch. It isn't a way to report
an expected problem: a command that can fail should `reject`, and pure code
should return a `Result`.
