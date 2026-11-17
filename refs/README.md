# refs/

Read-only clones of projects worth studying. The clones are git-ignored; only
this README, [repos.txt](repos.txt) and [sync.sh](sync.sh) are tracked.

```bash
refs/sync.sh              # clone or update everything enabled in repos.txt
refs/sync.sh jax hydro    # just these
```

- **Add a repo:** add or uncomment a line in `repos.txt`, then `refs/sync.sh <name>`.
- **Pin a version:** put a branch, tag or commit hash in the `ref` column.
- **Monorepos:** list directories after the ref to sparse-check-out only those
  (see the `mlir` line).
- Clones are shallow and get reset on every sync, so don't keep edits in them.
- Searches from the project root skip `refs/`. Search a clone by path,
  e.g. `rg "def jaxpr" refs/jax`.
- Restate (BSL), Convex (FSL) and Electric (commercial) are source-available:
  read them, don't copy from them.

## Tier 1: what to read, and why

The running example is a simple CRUD app. For each repo, the question is what
it teaches about expressing one, compiling it across machines, and testing it.

| Repo | Why it matters | Read first |
|---|---|---|
| **hydro** | Closest prior art: a Rust compiler stack for distributed programs. It has typed *locations* (process, cluster), *live collections* typed by boundedness and ordering, lattices, a simulator, and proofs. | `docs/docs/hydro/`, `hydro_lang/src/location/`, `hydro_lang/src/live_collections/`, `hydro_lang/src/sim/`, `design_docs/` |
| **jax** | The model for the architecture: a small IR (jaxpr), primitives that carry per-transformation rules, tracing, and sharding annotations the compiler propagates. | `docs/autodidax.md` (builds JAX from scratch; start here), `docs/tracing.md`, `docs/jax.sharding.rst`, `jax/_src/core.py` |
| **triton** | A Python front end that captures the function's AST instead of tracing it, so ordinary `if`/`for` work. `constexpr` marks build-time values. | `python/triton/compiler/code_generator.py`, `python/triton/language/core.py` |
| **wing** | "Infrastructure from code": abstract cloud resources with one implementation per target, *including a local simulator target*. Splits code into build-time and run-time ("preflight" and "inflight"). | `docs/docs/02-concepts/`, `packages/@winglang/sdk/src/cloud/` vs `target-sim/` vs `target-tf-aws/` |
| **encore** | Builds an app model (services, APIs, databases, pub/sub, crons) by statically parsing ordinary Go/TS code, validates it, then provisions it per cloud. | `v2/app/` (app model and validation), `v2/parser/` |
| **dbos-py** | A Python decorator API (`@workflow`, `@step`, `@transaction`) for durable execution on Postgres. A reference for how commands should read in Python. | `dbos/_dbos.py` (decorators), `dbos/_core.py`, `dbos/_recovery.py` |
| **tigerbeetle** | The gold standard for deterministic simulation testing (VOPR): simulated network and storage, fault injection, a state-machine model checked against the real one. | `src/vopr.zig`, `src/testing/` (`packet_simulator.zig`, `storage.zig`, `state_machine.zig`) |
| **maelstrom** | Small. Checks distributed systems against named consistency models: our "does the distributed run refine the logical program" check. | `doc/` (in order; `04-crdts/` and `06-raft/` most relevant), `doc/workloads.md` |

## Language design: effects and platforms

Ideas to borrow for a weft language, not foundations to build on.

| Repo | Why it matters | Read first |
|---|---|---|
| **roc** | *Platforms*: a pure app plus a host that provides every effect, our "targets as capability sets". Effectful functions are marked with `!` and `=>`. Structural tag unions for errors, in-place updates through reference counting. Pre-0.1, compiler mid-rewrite to Zig. | `docs/langref/platforms.md`, `functions.md` (purity), `tag-unions.md`, `errors.md`; `test/fx/` (an app plus its platform) |
| **koka** | Inferred *effect types* (what each function touches) and effect *handlers* (same program, different interpretation). Perceus reference counting and functional-but-in-place updates. | `doc/spec/book.kk.md` (effects and handlers chapters), `samples/handlers/`, `samples/learn/handler.kk`, `samples/learn/fip.kk` |

## Multitier languages and verified distribution

Prior attempts at "one program for client, server and database", and at
compiling away network faults. Papers for each are in
[docs/reading-list.md](../docs/reading-list.md).

| Repo | Why it matters | Read first |
|---|---|---|
| **urweb** | One statically typed program compiled to a C server, a JS client and SQL. The most complete multitier language. Quiet since 2024. | `doc/intro.ur`, `demo/crud.ur` (plus `crud1`–`crud3`), `demo/chat.ur` (server push), `doc/manual.tex` |
| **scala-loci** | *Placement types*: every value's type says which tier it lives on, and the compiler generates the communication. Embedded in Scala. Quiet since 2024. | `README.md`, `language/shared/src/main/scala/loci/language/`; tutorials are on the project site, scala-loci.github.io |
| **electric** | Electric Clojure v3: client and server code interleave in one function, and the compiler splits it and streams between them. Commercial license (free for non-commercial use): read, don't copy. | `Readme.md`, `docs/electric-protocol.md`, `src/hyperfiddle/electric3.cljc` |
| **verdi** | Coq framework where you verify a system on a perfect network, then *verified system transformers* make it tolerate faults. The closest formal analog to "write it as if on one machine". | `README.md`, `theories/Core/Net.v` (network semantics), `theories/Systems/SeqNum.v` (a transformer that handles duplicates), `PrimaryBackup.v`, `LockServ.v` |

## Owning the whole stack

| Repo | Why it matters | Read first |
|---|---|---|
| **darklang** | The new version of Darklang ("dark-next", not yet production-ready): a functional language for backends and CLIs, interpreted by an F# runtime. Code is stored as immutable, content-addressed functions with version control built in. Edits re-bind names, so running servers follow them live. Effects are inferred and gate permissions per code version. Recorded runs can be paused, resumed and forked. One runtime on one machine: no client and no distribution. See [docs/prior-art.md](../docs/prior-art.md). | `docs/effects-and-permissions.md` (inferred effects), `docs/live.md` (live updates as name re-binding), `docs/internals.md` (the code store and its version control), `docs/processes.md` (scheduler, traces, resume and fork), `docs/agent-traps.md` and `AGENTS.md` (writing a new language with AI agents) |

Later tiers are commented out in `repos.txt`, grouped by the question they answer:
- **Tier 2:** sync and offline, durable execution, placement, simulation libraries.
- **Tier 3:** hand-written baselines to benchmark against. For CRUD, RealWorld
  and TodoMVC each give one app spec with many hand-written implementations.
- **Tier 4:** compiler infrastructure: xdsl, MLIR, ibis.
