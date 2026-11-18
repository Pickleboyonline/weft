---
sidebar_label: The promise and its limits
description: Exactly what weft is designed to guarantee, what the simulator injects, and where the guarantees stop.
---

# The promise and its limits

:::note[A design, not yet a language]
This page states what weft is *designed* to guarantee. Nothing is implemented
yet, so none of it has been tested against a real compiler.
:::

## The promise

> Every run of a deployed weft app can be explained by a run of the
> one-machine program: the same commands, applied one at a time, in some
> order. Once the network goes quiet, every screen shows what the server
> holds.

The *one-machine program* is your code deployed as `local`: one
process, in memory, with no network. It's the reference everything else is
compared against.

The deployed app has one fixed shape: a server with a database, and any
number of clients. Clients reach the server only through
[queries and commands](/docs/reference/commands-and-queries). Spreading data
beyond that one server is an open question, and the promise doesn't cover it
(see [The limits](#the-limits)).

## What that means

These are the properties the design aims for:

- **Commands are atomic.** A command takes effect entirely or not at all.
  Returning `Err` undoes everything it wrote.
- **Commands appear to run one at a time.** The server's data always matches
  some one-at-a-time order of every client's commands.
- **Retries apply once.** Every command call carries a request id, and the
  server applies each id at most once, so a double click or a resent request
  doesn't run twice.
- **Writes refresh reads.** When a command writes a table, every query on
  screen that reads that table is refreshed. Your own change appears once the
  server confirms it; changes aren't shown before that.
- **Screens catch up.** Each query result is its own snapshot, so two results
  on one screen may briefly come from different moments. Once messages stop
  flowing, every query result a client shows equals what that query returns
  on the server.
- **Your checks hold.** Every check (in `checks/`) holds on the server after
  every step.
- **Failures are delivered.** A command that fails on the server comes back to
  the client as its typed reason (`Err(NoSuchList)`), and the client must
  handle it.

The full list, with the bug each one removes, is in
[Commands and queries: guarantees](/docs/reference/commands-and-queries#guarantees).

## What the simulator injects

Fault model as designed. "Specified" means a
[`Simulation`](/docs/reference/deployment#simulations) has a field for it;
"planned" means the design calls for it but doesn't specify it yet.

| Fault | What happens | Status |
|---|---|---|
| Message loss | A percentage of messages are dropped (`drop: 10%`) | Specified |
| Delay and reordering | Each message gets a random latency (`latency: 5ms..=2s`), so messages overtake each other | Specified |
| Duplicated messages | A message is delivered more than once (`duplicate: 1%`) | Specified |
| Offline periods | A client can't reach the server for a share of the run (`offline: 30%`) | Specified |
| Crashes and restarts | A server or client stops and recovers from durable state (`crashes: Crashes { … }`) | Specified |
| Extern failures | Each extern's simulation model decides how it fails (`Fallible { ok: 95%, error: Bounced }`) | Specified |
| Other services | Responses from hosts reached with `http` or `ws` come from a model per host (`outside: { … }`) | Specified |
| Clock skew | Machines disagree about the time | Planned |

Every simulated run is checked three ways:

1. **Your checks**, on the server's data after every step.
2. **The oracle:** the run's final data must match some one-at-a-time order of
   the same commands on the one-machine program.
3. **Convergence:** once the simulated network goes quiet, every query result
   a client shows must equal what that query returns on the server.

The simulator is deterministic. All randomness (the network, extern models,
`random` and `now`) comes from one seed, so a failing seed replays the same
run exactly.

## The limits

### Distribution isn't designed

The promise holds for one server and its database. Keeping copies of data on
clients, writing while offline, and spreading tables across several servers
are deliberately left open (see [the language design](/design/language),
open decision 4).

Once data is copied, two devices can each do something that is fine alone but
wrong together. No system can prevent that without the devices coordinating
first; this is the CALM theorem. Typical cases:

- uniqueness ("no two lists have the same name");
- limits ("at most 10 items per list");
- references ("every item belongs to a list that exists").

With one server, these hold, because commands run as if alone. Getting that
right is still work for the compiler: see the delete race in
[The big idea](/docs/#4-races-are-found-before-users-find-them).

### The simulator explores; it doesn't prove

A clean simulation means no failure in the runs it tried. You choose how many
runs and which faults. It's strong evidence, not a proof. A simulation can
also [exhaust](/docs/reference/checks-and-tests#how-strongly-a-check-is-verified)
a small configuration, which finds every failure within its bound and says
nothing beyond it.

### Externs are only as good as their models

The simulator runs an extern's `sim` model, not the real email service or
payment API. If the model is wrong, the simulation is wrong in the same way.

### Intent can't be inferred

If two devices each flip a checkbox, a `toggle` command flips it twice and
leaves it where it started. That's correct, since it's what one machine would
do with two toggles, but probably not what either person meant. A
`set_done(id, True)` command says what was meant. weft guarantees
correctness relative to the commands you write; choosing commands that say
what people mean is up to you.

### Who may call what isn't designed

Authorization is open. Until it's designed, every client can call every query
and command.

### Performance isn't promised

The goal is to come close to a hand-built system for common patterns. Nothing
here guarantees it, and nothing has been measured. The one firm rule is that
a query compiles to one database query per level of nesting, or doesn't
compile.

## Further reading

- Hellerstein, Alvaro. [Keeping CALM](https://doi.org/10.1145/3369736) (CACM
  2020): which logic needs coordination at all.
- [TigerBeetle's safety page](https://docs.tigerbeetle.com/concepts/safety/):
  a fault model stated precisely, which this page follows.
- [Prior art](/design/prior-art): who else has tried this, and what happened.
