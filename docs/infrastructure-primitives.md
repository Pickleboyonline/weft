# Infrastructure as primitives

**Status:** concept note, not a decision. A candidate design for weft's
physical layer.

Developers usually reach for infrastructure by name: "this needs a time-series
database," "put Redis in front of it," "use Kafka." Weft should never ask for
that. Look at what each of these systems *does* and they all turn out to be
combinations of a small number of independent design choices. Weft makes those
choices primitives, works out which ones a program needs from its code, and
leaves the product names to the platforms.

This is the infrastructure counterpart of JAX never naming hardware (see
[jax-analogy.md](jax-analogy.md)).

## The theory behind it

Each of these is in [reading-list.md](reading-list.md):

- **The periodic table of data structures** (Idreos et al.). Data structures
  are compositions of a few design primitives (partitioning, ordering, layout,
  fan-out, indexing), each with a cost model. The same group synthesized new
  structures this way, including Cosine, a key-value store designed to a
  cloud-cost budget.
- **The RUM conjecture** (Athanassoulis et al.). A data layout can optimize at
  most two of **R**ead cost, **U**pdate cost and **M**emory (space) overhead.
  Every storage engine picks a point in that triangle.
- **Differential dataflow and incremental view maintenance** (McSherry et al.).
  Caches, materialized views, rollups and stream processors are all *derived
  data kept up to date*; they differ only in when and how they refresh.
- **Streams and tables are duals** (Kleppmann). A table is a log folded into
  state, and a log is a table's history.
- **Consistency models and PACELC** (Adya; Viotti and Vukolić; Abadi). What
  replication can promise, and what each promise costs in latency even when
  the network is healthy.

## The primitives

A candidate set, intended to be the physical layer's equivalent of StableHLO:

| # | Primitive | Choices |
|---|---|---|
| 1 | **Representation** | Row or column; B-tree (sorted, updated in place), LSM tree (log-structured, write-optimized) or hash; encoding and compression |
| 2 | **Partitioning** | By key range, by hash, or by time; how many partitions |
| 3 | **Indexes** | Secondary, inverted (text search), spatial, vector (approximate nearest neighbor) |
| 4 | **Materialization** | Compute on read, maintain incrementally, or refresh in periodic batches |
| 5 | **Tiering** | Memory, local SSD, network disk, object store, edge; eviction; time-to-live |
| 6 | **Replication** | How many copies; with or without a leader; quorum sizes; synchronous or asynchronous |
| 7 | **Durability** | When a write counts as done: in memory, flushed to disk, replicated, copied across regions |
| 8 | **Movement** | Request/response, push (change feed), log shipping, bulk transfer |
| 9 | **Compute scheduling** | Inline in the request, background, batch, streaming, scheduled; fixed or elastic capacity |

## Familiar systems, decomposed

| What people call it | What it is, in primitives |
|---|---|
| Postgres | Row layout + B-tree indexes + durable on flush + single-leader replication + transactions |
| Time-series database | Partitioned by time + columnar + LSM (append-optimized) + incremental rollups + tiering by age |
| Redis as a cache | Memory tier + a materialized view + time-to-live and eviction + relaxed freshness |
| Kafka | Append-only log + hash partitioning + replication + push movement with consumer cursors |
| Elasticsearch | An incrementally maintained inverted index + sharding + eventual freshness |
| S3 | Object-store tier for large, immutable values: very durable, high latency |
| CDN | An edge-tier copy of immutable or versioned data |
| Data warehouse | Columnar + compression + batch compute kept separate from storage |
| Vector database | An incrementally maintained approximate-nearest-neighbor index |
| Lambda, workers, cron | Compute scheduling in its elastic, background and scheduled modes |

## Reading the workload from the code

Most of what decides these choices is visible in the program, much as XLA
reads tensor shapes from a program. An analytics table added to the todo app:

```weft
events = Table(Event)                    # only ever insert!-ed

daily_active = || events
  .group_by(|e| e.at.date())
  .map(|g| { day: g.key, users: g.items.distinct_by(.user).count() })
```

| Signal in the code | What the compiler concludes |
|---|---|
| Only `insert!` is ever called on `events` | Append-only: no in-place updates, so LSM or columnar storage is fine |
| Queries group by a time field | Partition by time |
| A query aggregates over a range | Maintain it incrementally, as a daily rollup |
| No query reads old events individually | Old partitions can move to cheap storage |

Together that's the design of a time-series database, and nobody named one.
What the code can't reveal is *size*: how many rows, how many writes per second.
That needs hints or production telemetry (see [jax-analogy.md](jax-analogy.md),
"Where the analogy breaks").

## Platforms supply the primitives

Each platform advertises which primitives it implements, at what cost, and with
which real guarantees, much as a PJRT plugin describes a device:

- **Postgres:** rows, B-trees, transactions, partitioning.
- **ClickHouse:** columnar storage and fast aggregation.
- **Redis:** the memory tier.
- **S3:** the object-store tier.

A *plan* assigns each table and query to primitives that the available
platforms provide, satisfying the program's behavior annotations and budgets.
`weft explain` shows the plan:

```
$ weft explain prod
events          Postgres, partitioned by day; partitions older than 30 days tiered to S3 (Parquet)
daily_active    incremental rollup table, updated per insert batch
todos           Postgres rows, B-tree on (owner, created); replicated to browser
```

Add ClickHouse to the deployment and the same source plans differently:
`events` moves to ClickHouse, and `todos` stays in Postgres.

In v1, existing systems are the kernels, as cuBLAS is for XLA. Generating
weft's own storage engines is the long-term counterpart of XLA's own codegen.

## The tradeoffs users choose

People shouldn't choose products, but some choices are genuinely theirs, because
they change what users experience. Each one is a point on a known tradeoff:

| Choice | Theory | Example annotation |
|---|---|---|
| Read cost vs write cost vs storage | RUM | `pins: { daily_active: Materialized }` |
| Freshness vs cost | Incremental view maintenance | `consistency: { daily_active: Eventual(1h) }` |
| Durability vs latency | Durability levels | `durability: { todos: Committed }` |
| Consistency vs latency | PACELC | `consistency: { todos: Session }` |
| Behavior when offline | Local-first, merge rules | `offline App: writes allowed` |

They fall into three kinds:

1. **Behavior annotations** change what users can observe, so they're part of
   the spec. The simulator uses them to decide which differences from the
   single-machine run are allowed.
2. **Budgets** (latency, payload size, rows sent to a client, money) are goals
   the compiler searches for a plan to meet. If no plan meets them, it reports
   which budget conflicts with which annotation.
3. **Pins** force a specific choice. The compiler warns if a pin breaks a
   budget.

```weft
prod = Deployment {
  machines:    { server: Machine { platform: "weft/server-rust-postgres" } },
  consistency: { todos: Session },
  durability:  { todos: Committed },
  budgets: {
    toggle!:      Budget { p99_latency: 50ms },
    daily_active: Budget { max_monthly_cost: 50 },
  },
  pins: { daily_active: Materialized },
}
```

## Open questions

1. **Is this the right primitive set?** Nine is a hypothesis. Search, blobs
   and vectors may need more than "an index".
2. **The cost model.** Budgets are only as good as the estimates behind them.
   How accurate does it need to be, and how is it checked against reality?
3. **Sizes.** Hints, telemetry, or both? Who supplies the hints?
4. **Plan changes are migrations.** Moving live data between representations
   without downtime has to be generated and simulated, like any other change.
5. **Platform fidelity.** Each platform's simulation model must match the real
   system's guarantees. How are those models tested?
6. **What v1 supports.** Probably SQLite and Postgres, a memory tier, and an
   object store, which is enough for the CRUD benchmark.
