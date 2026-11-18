---
title: Externs, jobs and endpoints
description: Code weft can't see into, scheduled work, and HTTP entry points for other systems.
---

# Externs, jobs and endpoints

These are declared as values of standard-library types, like tables and
deployments: a name, `=`, and a record saying how it behaves.

## Externs

An **extern** is a function weft can't see into: a payment API, an email
service, an existing library. You declare its type and how it behaves in
simulation; its implementation is host code.

```weft
send_email! : Text, Text => {} ! <reject [Bounced]>
send_email! = Extern {
  needs: [Email],
  sim:   Fallible { ok: 95%, error: Bounced, latency: 50ms..=2s },
}
```

- **The signature is required,** including its full effect row: the compiler
  can't infer anything about code it can't see.
- **`needs`** lists the capabilities the extern uses. It can only run on a
  machine whose platform provides them all.
- **`sim`** says how it behaves in simulation, and is required. Every escape
  hatch must have a model.
- **The implementation** goes beside the declaration, in the platform's host
  language: `externs/send_email.rs` for a Rust server, `externs/send_email.ts`
  for a TypeScript one.

### Simulation models

| Model | Behaves like |
|---|---|
| `Constant { value: v, latency: r }` | always returns `v`, after a latency in range `r` |
| `Fallible { ok: p, error: e, latency: r }` | succeeds with probability `p`, otherwise rejects with `e` |
| `Script(f)` | `f`, a weft function you write, called with the extern's arguments and a seeded random source |

## Jobs

A **job** runs a function on a schedule, on the server.

```weft
purge_done! : () => {} ! <..>
purge_done! = || items.delete_where!(|i| i.done and i.created < now!() - 30d)

purge = Job { every: 1h, run: purge_done! }
nightly_report = Job { at: daily(3h), run: send_report! }
```

- `every:` takes a `Duration`. `at:` takes a calendar schedule:
  `daily(time_of_day)`, `weekly(Monday, 9h)`, or `cron("0 3 * * *")`.
- `run:` takes a `!` function with no parameters. Each run is a transaction,
  like a command.
- Jobs live in `jobs/`. The simulator runs them on simulated time, so a job
  that breaks a check is caught like any other bug.

## Endpoints

An **endpoint** lets a system that isn't a weft client (a payment provider's
webhook, a mobile app, a script) call into the app over HTTP.

```weft
stripe_webhook = Endpoint {
  method: Post,
  path:   "/hooks/stripe",
  run:    handle_stripe_event!,
}
```

- `run:` takes a `!` function from a `Request` to a `Response`.
- Endpoints live in `endpoints/`. weft clients never need them: they call
  commands directly.
