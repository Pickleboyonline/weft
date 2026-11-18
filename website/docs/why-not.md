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
- Effects, purity and failures can be enforced by the type system, not by
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

**Not decided.** Today there's one answer, so there's nothing to choose:
shared tables, queries, commands and jobs run on the server, and components
on the clients. A helper runs wherever its callers do, and one that reads a
server table can only run on the server.

Placing data anywhere else (on several servers, or as copies on clients) is
the distribution question, which is deliberately open. Choosing placement
automatically would need a cost model and real usage data, and a wrong
automatic choice is hard to debug; Hydro, the closest research project, ships
explicit placement. See [the language design](/design/language).

## Why can't the client call the server directly?

**It does, but the call is sent, not run.** In a handler, `add_todo(draft)`
sends the command to the server, and `.await` waits for its result. Other
handlers keep running meanwhile. Underneath, handlers compile to state
transitions that return commands as values, as in Elm.

The cost is one rule to learn. In return, every effect is visible to the
compiler and the simulator, and a handler can be tested without a network.
The screen updates when the server confirms. Showing a change before that
(an optimistic update) is part of the client cache design, which is still
open.

## Why not write clients the way Elm does?

**By design: weft uses Elm's model with Svelte's feel.** In Elm, every
interaction needs a message variant plus a branch in one `update` function,
and all state lives in one global model. That's correct but heavy: inline
editing of one item touches six places.

weft components have local `state`, assigned with plain `=` in ordinary
handler functions, with `bind:` for inputs and scoped `style`, as in Svelte.
The compiler finds every assignment and generates the messages and the
`update` function, so the result is still the Elm Architecture: pure
transitions, messages as data, replayable runs.

The tradeoff is the one Elm warns about: state is spread across components
instead of sitting in one model. The compiler still keeps every component's
state in one tree, so logging, replay and debugging see all of it; only the
source is spread out.

## Why are there no `command` or `query` keywords?

**By design.** Queries and commands are values, `Query(…)` and `Command(…)`,
like `Table(…)` and `Job { … }`. A keyword per concept would grow the language
with every new kind of thing (jobs, webhooks), and an earlier draft had over
40 of them.

They are explicit, though. Two later drafts inferred them from effects and
use: a function a handler calls that writes was a command. That was dropped,
because queries and commands are the boundary between client and server: the
app's API, where validation and compatibility live, and the one place a
reader most needs to see. `weft explain api` lists them all.

## Why can't a query write?

**By design.** A query is re-run whenever a command writes a table it reads,
and its results are cached and shared by every component that calls it with
the same arguments. A write inside one would run an unpredictable number of
times. A `Query` that writes is a compile error.

## Why are server reads called queries, not views?

**By design.** weft used to call them views, which collided twice: with a
component's `view` (its markup), and with what "views" means to anyone coming
from Rails or most web frameworks (UI templates). Now a query is a read of
server data, and `view` means markup.

## Why does every project have the same layout?

**By design.** The compiler sees the whole program, so the layout can't change
what a program means. A fixed layout makes every project predictable, for
people, AI agents and generators alike, and lets conventions supply defaults:
the reference deployment is generated, and a file's path under `ui/pages/` is
its URL. `weft explain` shows every default it applied, so nothing is hidden.

## Why no exceptions?

**By design.** Failures are values, as in Roc. A command that can fail
returns a `Result` with a tagged reason (`return Err(EmptyTitle)`), and
returning `Err` also undoes everything it wrote. The reasons are part of the
command's type, and a client that calls it must handle them
(`add_todo(…).await.on_err(|err| …)`); the compiler checks that it does.

The cost is more handling code. In return, failures can't surprise the UI, and
adding a new failure reason flags every place that has to deal with it.

## Why doesn't weft work offline?

**Not decided.** With one server, every command is atomic and runs as if
alone, so the server's data always matches some one-machine run. Offline
writes break that: two devices can each do something fine alone but wrong
together, and some invariants (uniqueness, limits, references between
tables) can only be kept if devices coordinate first. That's a theorem, not a
missing feature. Rather than pick an answer before its consequences are
worked out, weft leaves copies of data on clients, offline writes and
consistency levels open. See [The promise and its limits](promise) and
[the language design](/design/language).

## Why only one server and browsers?

**By design, for now.** It's the most common shape for a CRUD app, and the one
whose guarantees can be stated and checked today. The machines are still a
record in a `Deployment`, not grammar: multitier languages that hard-coded
client and server couldn't grow past it. Other shapes (several regions, edge
nodes, devices that sync peer-to-peer) need the distribution design, which is
open.

## Why not use CRDTs for everything?

**Not decided.** A CRDT (a data type whose concurrent changes always merge)
guarantees that copies converge, but convergence isn't the same as
correctness. A merge can produce data your commands would never have produced
on one machine. In the current design, clients keep no copies of shared
data, so nothing needs merging. An earlier draft proposed replaying a client's pending commands on
the server by default, with merge rules as an opt-in. It isn't part of the
language: see open decision 4 in [the language design](/design/language).

## Why isn't there a weft editor?

**By design: no.** weft uses ordinary editors: a VS Code extension provides
highlighting, and a language server provides errors, hover and completion in
any editor. Darklang built its own editor and later abandoned it: it cut users
off from their usual tools, and AI coding tools made it obsolete.

## Why does weft simulate instead of proving things?

**By design.** A deterministic simulator works on any program, including ones
too large to prove anything about, and when it fails it hands you a
reproducible run. A simulation can also exhaust a small configuration (every
order of a few actions for a few clients), which finds most concurrency bugs
for certain. Proof tools such as Verdi show full proofs can be done for
specific systems, at a cost no one would pay for a todo app. How weft could
let you prove individual checks is an open question: see
[Testing and proof](/design/testing-and-proof).
