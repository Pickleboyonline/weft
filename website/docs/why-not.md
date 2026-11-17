---
sidebar_label: Why doesn't weft…?
description: Design decisions, each with its tradeoff. Some are firm; some are still open, and say so.
---

# Why doesn't weft…?

Each answer gives the decision and its tradeoff. **By design** marks a firm
decision. **Not decided** marks an open question, with the current leaning.

:::note[A design, not yet a language]
These are decisions about a design. Nothing here is implemented yet.
:::

## Why a new language instead of a library?

**Not decided.** The current design is a new language, but weft could equally
start as a library embedded in an existing language. Both would produce the
same intermediate representation, so the decision can be made later, or both
can exist.

**For a new language:**
- Effects, purity and rejections can be enforced by the type system, not by
  convention and lint rules.
- Client code, server logic and deployment share one coherent syntax.
- weft is a personal tool, and enjoying the language you write every day
  counts.

**Against:**
- The projects that got traction embedded themselves in an existing language:
  Temporal, Convex, Encore, Zero, Hydro, and JAX itself. The new-language
  attempts mostly stalled; Unison survived but took about a decade. See
  [Prior art](/design/prior-art).
- AI coding agents have never seen weft. If they write most of the code, that
  costs something every day.
- A parser, type checker, formatter, language server and error messages are
  most of the work of a language, before any of the distributed-systems work.

## Why doesn't weft decide where code runs?

**By design, for now.** Placement is written in `deploy.weft`. Choosing it
automatically needs a cost model and real usage data, and a wrong automatic
choice is hard to debug. Hydro, the closest research project, made the same
call and ships explicit placement. Automatic *suggestions* may come later.

## Why can't the client call the server directly?

**By design.** Client code changes state only inside actions, and an action
sends a command with `do add_todo!(draft)`: the command goes out after the
action finishes, not in the middle of it. Underneath, actions compile to pure
functions that return commands as values, as in Elm.

The cost is one rule to learn. In return, every effect is visible to the
compiler and the simulator, optimistic updates come for free (the command can
run against the client's local copy first), and an action can be tested
without a network.

## Why not write clients the way Elm does?

**By design: weft uses Elm's model with lighter syntax.** In Elm, every
interaction needs a message variant plus a branch in one `update` function,
and all state lives in one global model. That's correct but heavy: inline
editing of one item touches six places.

weft clients are components with local `state`, changed by named `action`s,
with `bind:` for inputs. The compiler generates the messages and the `update`
function from the actions, so the result is still the Elm Architecture: pure
transitions, messages as data, replayable runs. Redux Toolkit does the same in
JavaScript.

The tradeoff is the one Elm warns about: state is spread across components
instead of sitting in one model. The compiler still keeps every component's
state in one tree, so logging, replay and debugging see all of it; only the
source is spread out.

## Why can't a view write?

**By design.** Views are recomputed whenever their data changes, and clients
subscribe to them. A write inside one would run an unpredictable number of
times, on unpredictable machines. Writing in a `view` is a compile error.

## Why no exceptions?

**By design.** A command fails with `reject` and a tagged reason
(`reject EmptyTitle`). The reasons are part of the command's type, and a
client that calls it must handle them (`do add_todo!(…) else Rejected`); the
compiler checks that it does.

The cost is more handling code. In return, failures can't surprise the UI, and
adding a new rejection reason flags every place that has to deal with it.

## Why doesn't weft guarantee everything behaves exactly like one machine?

**Because it can't.** With offline writes, some invariants (uniqueness, limits,
references between tables) can only be guaranteed if devices coordinate first.
That's a theorem, not a missing feature. weft promises to say which
invariants are affected and make you choose how to handle them. See
[The promise and its limits](promise).

## Why only one server and browser clients, to start?

**By design, for the first version.** Every additional topology (peer-to-peer,
several regions, edge nodes) multiplies what has to be built and tested.
Multitier languages and infrastructure-from-code tools that tried to cover
everything at once struggled to finish any of it. Other topologies come after
this one works.

## Why not use CRDTs for everything?

**Not decided; current leaning: replay by default, merge rules opt-in.** A
CRDT (a data type whose concurrent changes always merge) guarantees that
copies converge, but convergence isn't the same as correctness. A merge can
produce data your commands would never have produced on one machine. By
default, weft replays a client's pending commands on top of the server's data,
which keeps the one-machine meaning. A table or field can opt in to a merge
rule when convergence is what you want.

## Why isn't there a weft editor?

**By design: no.** weft uses VS Code (an extension already provides syntax
highlighting) and, later, a language server that works in any editor.
Darklang built its own editor and later abandoned it: it cut users off from
their usual tools, and AI coding tools made it obsolete.

## Why does weft run a simulator instead of proving things?

**By design, for now.** A deterministic simulator works on any program,
including ones too large to prove anything about. When it fails, it hands you
a reproducible run. Proof tools such as Verdi (for Coq) show it can be done
for specific systems, at a cost no one would pay for a todo app. Exhaustive
checking of small configurations is a possible middle ground later.
