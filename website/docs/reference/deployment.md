---
title: Deployment and simulation
description: Deployment and Simulation values, and the defaults conventions supply.
---

# Deployment and simulation

Where the program runs, and how it's tested, are values in their own files.
Nothing outside `deploy/` and `simulate/` changes between deployments.

## Deployments

A deployment is a value of type `Deployment`, in `deploy/`:

```weft
/// A server with Postgres, and any number of browsers.
prod = Deployment {
  machines: {
    server:  Machine { platform: "weft/server-rust-postgres" },
    browser: Machine { platform: "weft/browser-ts", count: Many },
  },
}
```

| Field | Type | Default |
|---|---|---|
| `machines` | a record of `Machine`s: one server, and the clients | required |
| `handlers` | a list of [effect handlers](effects#user-defined-effects) to install for every command | none |

| `Machine` field | Type | Default |
|---|---|---|
| `platform` | `Text` | required |
| `count` | `One`, `Fixed(n)` or `Many` | `One` |

Shared tables, queries, commands and jobs run on the server; components run
on the clients. Spreading data across several servers, keeping copies of it
on clients, and offline use aren't designed yet (see
[the language design](/design/language)).

### Defaults

- **`local` is generated.** Every project has a deployment named `local`: one
  process, in memory, with no network. It's the reference the simulator
  checks every other deployment against, so it's never written.
- `weft explain prod` prints every default it applied, and why.

## Simulations

A simulation is a value of type `Simulation`, in `simulate/`:

```weft
/// Three browsers on bad wifi: dropped messages, offline a third of the time.
flaky_wifi = Simulation {
  clients:            3,
  actions_per_client: 500,
  network: Network { drop: 10%, latency: 5ms..=2s, offline: 30% },
}
```

| Field | Type | Default |
|---|---|---|
| `deployment` | a `Deployment` | `prod` |
| `clients` | `Int` | `2` |
| `actions_per_client` | `Int` | `100` |
| `network` | `Network` | no faults |
| `crashes` | `Crashes` | none |
| `exhaust` | `{ actions_per_client: Int }` | off; see [Checks](checks-and-tests#how-strongly-a-check-is-verified) |
| `runs` | `Int`: how many seeds to try | `100` |
| `outside` | `List(Outside)`: how other services behave, one `Outside { host, model }` per host | none; a call to a host with no model is reported |

| `Network` field | Meaning |
|---|---|
| `drop` | share of messages lost |
| `duplicate` | share of messages delivered twice |
| `latency` | a `Range(Duration)` each message's delay is drawn from |
| `offline` | share of the run each client spends unable to reach the server |

`Crashes { server: 1%, client: 5% }` restarts a machine from its durable state
at that rate per step.

**Other services** reached with `http` or `ws` are modeled per host, with the
same [models](resources#simulation-models) as externs, or `Script(f)` to answer
from weft code:

```weft
outside: [
  Outside { host: "api.example.com", model: Fallible { ok: 99%, error: Timeout, latency: 20ms..=800ms } },
  Outside { host: "prices.example.com", model: Script(fake_prices) },
]
```

Responses from other services are input to the run, not part of the app's
state: the one-machine comparison takes them as given.

**What a simulated client does:** it acts like a person. At each step it picks
one of the handlers attached to something on its screen, or types random text
(including empty text) into a bound input, or follows a link. It never
produces an action no person could.
