# The JAX analogy

**Status:** concept note, not a decision. It explains how JAX copes with many
kinds of hardware and what weft takes from that.

Weft's pitch is "JAX for general software engineering." The analogy holds at a
specific point: **JAX programs never name hardware, and weft programs should
never name infrastructure.** A JAX program says "multiply these matrices," not
"run cuBLAS on GPU 3." A weft program should say "keep this data and run this
query," not "use Postgres with a Redis cache." In both cases the compiler and
its backends make the physical choices.

## How JAX gets from a program to hardware

Each layer decides a little more about *how*, and the program never changes:

| Stage | What it does | Weft equivalent |
|---|---|---|
| **jaxpr** | Your traced program: pure operations, with no hardware anywhere | The logical program |
| **StableHLO** | The narrow waist: a fixed, versioned set of around a hundred hardware-independent ops (dot, convolution, reduce, gather, scatter, elementwise, collectives). Every front end emits it; every backend consumes it. | The logical IR's primitive set |
| **Sharding (Shardy)** | You annotate a few arrays with how to split them across a *mesh* of devices. The compiler propagates that to the rest of the program and inserts the communication (all-reduce, all-gather…). Shardy replaced the older GSPMD and is now always on. | Placement annotations, with sync protocols inserted wherever data crosses machines |
| **Fusion** | Merges consecutive ops into one kernel, so intermediate results never touch slow memory | Pushing filters and joins into one query; no N+1 queries |
| **Layout assignment** | Chooses each array's physical memory layout. Layout is never part of what the program means. | Row vs column, sorted vs log-structured, how data is partitioned |
| **Buffer assignment and memory kinds** | Decides what lives in fast memory, what gets reused, and what is offloaded to host memory | Storage tiers: memory, SSD, object store, edge |
| **Codegen or library calls** | Generates its own kernel, or calls a vendor library (cuBLAS, cuDNN) when that's faster. On GPUs, autotuning benchmarks the candidates and keeps the fastest. | Generate our own storage code, or delegate to an existing system used as a "library kernel" |
| **PJRT** | A plugin API. A hardware vendor adds a new device by implementing an XLA lowering plus PJRT, with no changes to JAX itself. | Platforms (see [language.md](language.md), section 8) |
| **Pallas, custom calls** | The escape hatch: write the kernel yourself | Externs and pins |

## The tradeoffs JAX still asks you to make

JAX automates the mechanics, but some choices are about your problem rather
than the code, so it exposes them as primitives. None of them names a vendor
library.

| JAX primitive | The tradeoff | Weft equivalent |
|---|---|---|
| Sharding annotations | How to split work across devices | `place … on …` |
| `jax.checkpoint` (remat) policies | Store intermediate values, or recompute them to save memory | Materialize a view, or compute it on read |
| Memory kinds (`device`, `pinned_host`) | Keep data in fast memory, or offload it to host memory | Storage tiering |
| Precision (e.g. bfloat16) | Lower precision for speed | Freshness or consistency relaxed for latency |

Where to read about these in `refs/jax`:
- `docs/about.md` ("A pluggable backend") describes XLA and PJRT.
- `docs/parallel.md` and `docs/shard_map.md` cover sharding.
- `docs/gradient-checkpointing.md` covers remat policies and offloading to `pinned_host`.
- `docs/pallas/` covers the kernel escape hatch.

## What weft takes from it

1. **A narrow waist.** One small, fixed set of primitives sits between every
   front end and every backend. For weft this is the logical IR, and the
   physical primitives in [infrastructure-primitives.md](infrastructure-primitives.md).
2. **Physical layout is never semantics.** Whether a table is stored by row or
   by column changes performance, not meaning, so it's the compiler's call.
3. **Existing systems are kernels.** XLA calls cuBLAS when cuBLAS wins. Weft v1
   should do the same with Postgres, SQLite, Redis and S3, and generate its own
   storage code only once a pattern is common enough to be worth it.
4. **Backends are plugins.** New hardware reaches JAX without changing JAX.
   New infrastructure should reach weft as a new platform, without changing
   the language or the compiler core.
5. **Tradeoffs are exposed as problem-level choices,** never as vendor
   choices.

## Where the analogy breaks

1. **No static shapes.** XLA knows every array's exact size at compile time.
   Weft can't know how many rows a table will have or how many writes per
   second it will get. It needs size hints (`expect events: ~10M rows/day`),
   re-planning from production telemetry the way a database's query optimizer
   uses statistics, or both.
2. **Re-planning moves data.** XLA can recompile for free. Changing a table
   from row storage to column storage means migrating live data without
   downtime, so every plan change is a migration the compiler must generate
   and simulate.
3. **The kernels have quirks.** cuBLAS behaves the same everywhere. Real
   databases differ in isolation levels, consistency guarantees and failure
   modes, and Jepsen has shown that many promise more than they deliver. Each
   platform's simulation model has to capture what the system *actually*
   guarantees, or the simulator checks against a fiction.
4. **Cost is money as well as time.** A budget can be "$200 a month" as well
   as a latency target, so the planner optimizes for both.

## Further reading

From [reading-list.md](reading-list.md):
- Frostig, Johnson, Leary, *Compiling Machine Learning Programs via
  High-Level Tracing* (the original JAX paper).
- Xu et al., *GSPMD* (how a few sharding annotations propagate to a whole
  program).
