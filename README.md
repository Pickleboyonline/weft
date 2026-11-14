# Weft

JAX for general software engineering. You write system logic in a restricted,
traceable model with explicit escape hatches. The compiler takes over
distribution, coordination, storage and testing. Built for the workflow where
a person or an agent writes the logic, and the compiler and simulator check it.

**Status:** scaffold / hello world, working toward milestone 1 (IR + simulator +
Python front end, enough to express and simulate the OTA protocol end to end).
Nothing deploys yet.

## Try it

```bash
make demo
```

That traces [examples/ota.py](examples/ota.py) into IR, validates and prints it,
runs one seed, and sweeps 500 seeds of the correct version (all pass) and of a
one-line buggy version (about half fail; you get the seed and a replay command).

```bash
make test
```

Needs Rust (cargo) and Python 3.10+. There are no Python dependencies.

## How it fits together

```
examples/ota.py ──trace──▶ build/ota.weft.json ──▶ weft-sim check | run | sweep
 (Python, build time)       (Weft IR v0: the product)   (deterministic simulator)
                                         └──▶ later: lowerings (Rust services, WASM, TS SDK, storage)
```

| Path | What |
|---|---|
| [python/weft/](python/weft/) | Front end. Traces decorated Python functions into IR, like `jax.jit` tracing. |
| [crates/weft-ir/](crates/weft-ir/) | IR types, validator, pretty-printer. The contract between front ends and backends. |
| [crates/weft-sim/](crates/weft-sim/) | Deterministic simulator and the `weft-sim` CLI. |
| [examples/ota.py](examples/ota.py) | Hello world: OTA firmware rollout to 5 devices over a lossy network. |
| [docs/ir-v0.md](docs/ir-v0.md) | IR semantics. |

## Design decisions in v0

- **The IR is the product.** The Python front end can be thrown away. A custom
  language later is just another front end that emits the same JSON (or a future
  MLIR dialect).
- **Handlers are straight-line SSA with guarded effects.** There are no loops or
  branches in the IR. `with weft.when(cond):` guards effects, and
  `weft.select` picks between values. Python `if`/`for` still work on
  *build-time* values, which makes Python the metaprogramming layer. Using
  `if` on a *traced* value raises an error that names the fix.
- **Locations are capability sets, not device types.** A device is any place
  with `clock`, `storage` and `flash`. State needs `storage`, timers need
  `clock`, and an extern's effects must be capabilities of its location.
- **Locations share no memory.** Reading another location's state is rejected
  (in Python and again in the validator). Data moves only by messages, which
  leaves the compiler free to place and replicate locations.
- **Escape hatches are typed externs.** An extern declares its effects, and its
  simulation model is *required*. Without one the program doesn't build.
- **Determinism.** One seeded SplitMix64 PRNG and one event queue ordered by
  `(time, seq)`. A failing seed is a complete, replayable bug report.

## Next steps toward milestone 1

1. **Crash/restart faults**, with state marked durable or volatile, so OTA can
   be tested for power loss during a flash.
2. **Maps and bounded loops** in the IR (e.g. the cloud tracking per-device
   versions), which lets the KV-store benchmark be expressed.
3. **Behavioral models written in the DSL** instead of the built-in
   `fallible`/`constant` list, so a model can depend on its arguments and keep state.
4. **Bounded liveness** ("converges within 30s of heal") in place of
   end-of-run `eventually`.
5. **Trace shrinking.** Shrink a failing seed's schedule to the fewest events
   that still fail.
6. **First lowering.** Cloud handlers become a Rust service (axum + sqlite). The
   simulator then acts as the reference semantics for differential testing.
