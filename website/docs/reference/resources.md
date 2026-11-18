---
title: Externs, jobs and endpoints
description: Host libraries weft can't see into, scheduled and triggered work, and HTTP entry points for other systems.
---

# Externs, jobs and endpoints

These are declared as values of standard-library types, like tables and
deployments: a name, `=`, and a record saying how it behaves.

## Externs

An **extern** is a function from an existing host library, which weft can't
see into: an image library, a PDF renderer, a vendor's SDK. You declare its
type and how it behaves in simulation; its implementation is host code.

Most code never needs one. Other services are reached with the platform's
`http` and `ws` modules, in weft (see
[Standard library: platform modules](stdlib#platform-modules)).

```weft
resize_image : Bytes, Int -> Result(Bytes, [NotAnImage])
resize_image = Extern {
  needs: [Images],
  sim:   Fallible { ok: 98%, error: NotAnImage, latency: 5ms..=200ms },
}
```

- **The signature is required:** the compiler can't infer anything about code
  it can't see. A call to an extern always counts as an effect.
- **`needs`** lists the capabilities the extern uses. It can only run on a
  machine whose platform provides them all.
- **`sim`** says how it behaves in simulation, and is required. Every escape
  hatch must have a model.
- **The implementation** goes beside the declaration, in the platform's host
  language: `externs/resize_image.rs` for a Rust server,
  `externs/resize_image.ts` for a TypeScript one.

### Simulation models

| Model | Behaves like |
|---|---|
| `Constant { value: v, latency: r }` | always returns `v`, after a latency in range `r` |
| `Fallible { ok: p, error: e, latency: r }` | succeeds with probability `p`, otherwise returns `Err(e)` |
| `Script(f)` | `f`, a weft function you write, called with the extern's arguments (or, for another service, the request) and a seeded random source |

## Jobs

A **job** runs a function on the server: on a schedule, or whenever a
query's result changes.

```weft
purge_done : () -> {}
purge_done = || items.delete_where(|i| i.done and i.created < now() - 30d)

purge = Job { every: 1h, run: purge_done }
nightly_report = Job { at: daily(3h), run: send_report }
```

- `every:` takes a `Duration`. `at:` takes a calendar schedule:
  `daily(time_of_day)`, `weekly(Monday, 9h)`, or `cron("0 3 * * *")`.
- `watch:` takes a query. The job runs whenever the query's result changes,
  and once at startup.
- `run:` takes a function with no parameters. If it writes tables, it's a
  command: atomic, and it can't wait. If it waits on the outside world, it
  changes tables only by calling commands, each of them atomic.
- Jobs live in `jobs/`. The simulator runs them on simulated time, so a job
  that breaks a check is caught like any other bug.

### Doing something outside after a change

A command can't call another service, because it's atomic and may run twice.
Record what needs doing in a table instead, and let a job do it:

```weft
/// Every welcome email not yet sent.
unsent = || welcomes.filter(|w| not w.sent)

send_welcomes = || {
  for w in unsent() {
    match http.post("https://mail.example.com/send", w.message, idempotency_key: w.id).await {
      Ok(_)  -> mark_sent(w.id)
      Err(_) -> {}                  // left unsent: the next run tries again
    }
  }
}

welcome_mail = Job { watch: unsent, run: send_welcomes }
```

- The command that signs someone up inserts a `welcomes` row in the same
  atomic run, so a welcome is recorded exactly when the signup is.
- Sending is **at least once**: if the server stops between the request and
  `mark_sent`, the next run sends again. Passing the row's id as an
  idempotency key, to a service that supports one, makes the service ignore
  the repeat.

## Endpoints

An **endpoint** lets a system that isn't a weft client (a payment provider's
webhook, a mobile app, a script) call into the app over HTTP.

```weft
stripe_webhook = Endpoint {
  method: Post,
  path:   "/hooks/stripe",
  run:    handle_stripe_event,
}
```

- `run:` takes a function from a `Request` to a `Response`.
- Endpoints live in `endpoints/`. weft clients never need them: they call
  commands directly.
