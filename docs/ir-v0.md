# Weft IR v0

Encoded as JSON (one file per program). The Rust types in
[crates/weft-ir/src/lib.rs](../crates/weft-ir/src/lib.rs) are the normative
schema, and `weft-sim check` validates a file and pretty-prints it.

## Program

| Section | Meaning |
|---|---|
| `locations` | `{name, replicas, caps}`. A place code runs. Each replica has its own state. |
| `messages` | `{name, fields: [[name, type]]}`. The only way data moves between locations. |
| `state` | `{name: "<loc>.<x>", location, type, init}`. A durable cell owned by one location. Needs `storage`. |
| `externs` | `{name, location, params, returns, effects, sim}`. An escape hatch. `effects` ⊆ the location's caps, and `sim` is required. |
| `handlers` | `{name, location, trigger, body}`. Triggers: `message` (from another location), `input` (from outside), `timer` (needs `clock`). |
| `properties` | `always`, `eventually` (a body plus a bool `result`, optionally `forall` a replicated location), or `monotonic` (an int state). |
| `scenarios` | Simulator harness: duration, network faults, heal time, timed inputs. Never lowered. |

Types: `int` (i64), `bool`, `loc` (a reference to one replica; no literals).

## Bodies

A body is a list of ops in SSA form. `id` is the op's index, and args refer only
to earlier ids.

| Op | Kind | Notes |
|---|---|---|
| `const {value}` | pure | |
| `field {name}` | pure | A field of the triggering message. |
| `sender` | pure | `loc` of the sending replica. Only in `message` handlers. |
| `read {state}` | pure | Handlers may only read their own location's state. |
| `bin {fn, args}` | pure | `add sub mul` (int), `lt le gt ge` (int → bool), `eq ne` (same type → bool), `and or` (bool). |
| `not`, `select` | pure | `select [cond, a, b]` |
| `call {extern, args}` | effect | Returns the extern's type. |
| `write {state, args}` | effect | |
| `send {to, message, fields}` | effect | `to` is `{"location": name}` (every replica) or `{"value": id}` (one replica). |

Effects may carry a `guard` (the id of a bool op). The effect happens only if the
guard is true. Nested `when` blocks in the front end become `and`ed guards.

## Execution semantics

- **Atomic handlers.** A handler runs to completion on one replica. Reads see
  writes made earlier in the same handler.
- **Sends** are buffered and leave when the handler commits.
- **Guarded-off calls** don't run and yield the type's zero (`0` / `false`).
- **Extern latency** holds the replica busy. Its sends leave late, and other
  events for that replica wait until it is free.
- **Network:** each message is dropped with `drop_rate` until `heal_at_ms`
  (always, if unset), and delayed by a uniform `latency_ms`. Messages can
  therefore reorder.
- **Properties:** `always` and `monotonic` are checked after every event,
  `eventually` at the end of the run. `forall L` evaluates the body once per
  replica of `L`. Reads of singleton locations see their single replica.
- **Determinism:** all randomness (network, sim models, timer phase) comes from
  one PRNG seeded per run. Events are ordered by `(time, insertion seq)`.
