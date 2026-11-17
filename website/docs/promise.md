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
> order, within the consistency you declared. When your choices make that
> impossible for one of your invariants, the compiler says which invariant,
> and why, instead of quietly weakening it.

The *one-machine program* is your code deployed with `deploy local`: one
process, in memory, with no network. It's the reference everything else is
compared against.

## What that means

These are the properties the design aims for under the default consistency
setting, `session`:

- **Commands are atomic.** A command takes effect entirely or not at all.
- **Commands appear to run one at a time.** The server's data always matches
  some one-at-a-time order of every client's commands.
- **You see your own changes immediately.** A client applies its own commands
  locally first, and the server confirms or rejects them later.
- **Clients converge.** Once messages stop flowing, every client's views equal
  the server's.
- **Your checks hold.** Every `check` holds on the server after every step.
  A client's copy is checked once its sync settles.
- **Rejections are delivered.** A command the server rejects comes back to the
  client as a typed reason (`Rejected(NoSuchList)`), and the client's
  optimistic change is rolled back.

What stronger or weaker settings guarantee is part of the open design.

## What the simulator injects

Fault model as currently designed. "Specified" means `deploy.weft` has syntax
for it today; "planned" means the design calls for it but hasn't specified it.

| Fault | What happens | Status |
|---|---|---|
| Message loss | A percentage of messages are dropped (`drop 10%`) | Specified |
| Delay and reordering | Each message gets a random latency (`latency 5ms..2s`), so messages overtake each other | Specified |
| Offline periods | A client is disconnected for a share of the run (`offline 30%`) and queues its commands | Specified |
| Extern failures | Each extern's `sim` block decides how it fails (`ok 95%, else Bounced`) | Specified |
| Duplicated messages | A message is delivered more than once | Planned |
| Crashes and restarts | A server or client stops and recovers from durable state | Planned |
| Clock skew | Machines disagree about the time | Planned |

Every simulated run is checked three ways:

1. **Your `check`s**, after every step on the server, and on each client once
   it has synced.
2. **The oracle:** the run's final data must match some one-at-a-time order of
   the same commands on the one-machine program.
3. **Convergence:** once the simulated network goes quiet, every client's views
   must equal the server's.

The simulator is deterministic. All randomness (the network, extern models,
`random!` and `now!`) comes from one seed, so a failing seed replays the same
run exactly.

## The limits

### Some invariants need coordination

With offline writes, two devices can each do something that is fine alone
but wrong together. No system can prevent that without the devices
coordinating first; this is the CALM theorem. Typical cases:

- uniqueness ("no two lists have the same name");
- limits ("at most 10 items per list");
- references ("every item belongs to a list that exists"; see the delete race
  in [The big idea](/docs/#4-distribution-bugs-are-found-before-users-find-them)).

For each one, weft is designed to make you choose rather than choose for you:

- **Reject on arrival.** The server checks when the command arrives, and the
  device that lost the race gets a typed rejection. `add_item!` does this with
  `NoSuchList`.
- **Require coordination.** The command waits for the server, so it doesn't
  work offline.
- **Merge.** Accept both and resolve with a rule you write.

### The simulator explores; it doesn't prove

A clean simulation means no failure in the runs it tried. You choose how many
runs and which faults. It's strong evidence, not a proof. Exhaustive checking
of small configurations, as Hydro does, may come later.

### Externs are only as good as their models

The simulator runs an extern's `sim` block, not the real email service or
payment API. If the model is wrong, the simulation is wrong in the same way.

### Intent can't be inferred

If two devices each flip a checkbox, a `toggle!` command flips it twice and
leaves it where it started. That's correct, since it's what one machine would
do with two toggles, but probably not what either person meant. A
`set_done!(id, True)` command says what was meant. weft guarantees
correctness relative to the commands you write; choosing commands that say
what people mean is up to you.

### Performance isn't promised

The goal is to come close to a hand-built system for common patterns. Nothing
here guarantees it, and nothing has been measured.

## Further reading

- Hellerstein, Alvaro. [Keeping CALM](https://doi.org/10.1145/3369736) (CACM
  2020): which logic needs coordination at all.
- [TigerBeetle's safety page](https://docs.tigerbeetle.com/concepts/safety/):
  a fault model stated precisely, which this page follows.
- [Prior art](/design/prior-art): who else has tried this, and what happened.
