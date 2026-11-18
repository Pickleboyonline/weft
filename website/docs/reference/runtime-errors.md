---
title: Runtime errors
description: What happens when a program fails at run time, and crash.
---

# Runtime errors

A runtime error is a bug: something the code didn't handle and the type
checker couldn't rule out. It's different from an `Err` result, which is an
expected outcome a function reports to its caller.

## What causes them

| Cause | Example |
|---|---|
| Integer overflow | an `Int` result beyond 64 bits |
| Division by zero | `n / 0`, `n % 0`, `x / 0.0` |
| An impossible state, declared with `crash` | see [`crash`](#crash) |
| A loop or recursion that doesn't finish within its time limit | a `while` whose condition never becomes false |

Out-of-range indexing isn't one of them: `list.get(i)` returns an `Option`.

## What happens

| Where | The effect |
|---|---|
| A command | Everything it did is undone, as when it returns `Err`. The error is reported to the developer. A handler awaiting the command stops with a runtime error too (see the next row), since the failure isn't one of the command's reasons. A command that can't reach the server isn't an error: it's retried until it arrives, and applied once. |
| A query | `status(query)` becomes `Failed(reason)`, and views reading it directly keep their last value. |
| A handler or task | Its state changes since it last reached an `.await` are discarded, and nothing it sent since then is sent. Its `defer` blocks run. A task isn't restarted until its component appears again or its key changes. |
| A job or endpoint | Everything its run wrote is undone, and the error is reported. An endpoint responds with status 500. |
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
an expected problem: anything that can fail, a command included, should return
a `Result`.
