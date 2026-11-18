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
## A server with Postgres, and any number of browsers.
prod = Deployment {
  machines: {
    server:  Machine { platform: "weft/server-rust-postgres" },
    browser: Machine { platform: "weft/browser-ts", count: Many, offline: True },
  },
  consistency: { lists: Session, items: Session },
}
```

| Field | Type | Default |
|---|---|---|
| `machines` | a record of `Machine`s | required |
| `placement` | a record from tables and components to machine names | tables on `server`, the client (`ui/App.weft`) on `browser` |
| `consistency` | a record from tables to a consistency level | `Session` for every table |
| `durability` | a record from tables to a durability level | `Committed` |
| `budgets` | a record from functions to a `Budget` | none |
| `pins` | a record from tables or queries to a storage choice | none: the compiler chooses |
| `handlers` | a list of [effect handlers](effects#user-defined-effects) to install for every command | none |

### Machines

| Field | Type | Default |
|---|---|---|
| `platform` | `Text` | required |
| `count` | `One`, `Fixed(n)` or `Many` | `One` |
| `offline` | `Bool`: can it keep working without a connection? | `False` |

### Consistency and durability

| Consistency | Each client sees… |
|---|---|
| `Strong` | every committed write, immediately. Commands wait for the server. |
| `Session` | its own writes immediately, other clients' after sync |
| `Eventual(max_staleness)` | everything within `max_staleness`, e.g. `Eventual(30s)` |

| Durability | A write is kept once… |
|---|---|
| `Committed` | it's on the server's disk |
| `Replicated(n)` | it's on `n` machines |

### Budgets and pins

```weft
budgets: {
  add_item!:    Budget { p99_latency: 50ms },
  daily_active: Budget { max_monthly_cost: 50 },
},
pins: { daily_active: Materialized },
```

- **A budget** is a limit the compiler's plan must meet. If no plan can, it
  reports which budget conflicts with which choice.
- **A pin** forces one storage choice for a table or query (`Materialized`,
  `Indexed([.field])`, `Replicated`, `ServerOnly`), overriding the compiler.

### Defaults

- **`local` is generated.** Every project has a deployment named `local`: one
  process, in memory, with no network. It's the reference the simulator
  checks every other deployment against, so it's never written.
- `weft explain prod` prints every default it applied, and why.

## Simulations

A simulation is a value of type `Simulation`, in `simulate/`:

```weft
## Three browsers on bad wifi: dropped messages, offline a third of the time.
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

| `Network` field | Meaning |
|---|---|
| `drop` | share of messages lost |
| `duplicate` | share of messages delivered twice |
| `latency` | a `Range(Duration)` each message's delay is drawn from |
| `offline` | share of the run each client spends disconnected |

`Crashes { server: 1%, client: 5% }` restarts a machine from its durable state
at that rate per step.

**What a simulated client does:** it acts like a person. At each step it picks
one of the handlers attached to something on its screen, or types random text
(including empty text) into a bound input, or follows a link. It never
produces an action no person could.
