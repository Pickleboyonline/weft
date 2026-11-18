# Prior art: who tried this, and what happened

Projects that tried some part of weft's idea (one program, with the system
around it handled for you), what they actually delivered, and what became of
them. [reading-list.md](reading-list.md) covers the research papers; this page
covers outcomes. Statuses were checked on 2026-11-17 from each project's own
posts and GitHub activity. Anything that couldn't be verified is marked as
such.

## Why this page exists

Weft is a personal tool. The goal is to build end-to-end systems that are
robust, correct and fast, without constantly engineering and re-checking the
work. So the failure modes that matter most here are the ones that stop a tool
from becoming usable at all, not the ones that stop it from becoming a
business. Section 4 separates the two.

## 1. Projects that stopped or changed direction

### Eve (2014–2018)

- **Pitch:** a relational language, a temporal database and an environment for
  non-programmers, all in one ([Two years of Eve](https://chris-granger.com/2016/07/21/two-years-of-eve/)).
- **Delivered:** 30+ prototypes. The first public release came about two years
  in. Mid-way, the team split the developer platform from the end-user
  interface, judging that hiding complexity behind UI hadn't worked.
- **Outcome:** wound down in January 2018 after finding no buyer or partner
  ([announcement](https://groups.google.com/g/eve-talk/c/YFguOGkNrBo)). There's
  no public post-mortem.
- **Lesson:** years of prototypes before anyone outside the team used the
  semantics, and a change of audience mid-way.

### Dark / Darklang (2017–present)

- **Pitch (2019):** one language, editor and infrastructure for backends, with
  edits going live without a deploy step ([Unveiling Dark](https://blog.darklang.com/unveiling-dark)).
- **What went wrong, in their words:**
  - 2020: they chased growth before the product fit, while basics such as
    auth, a package manager and team support were missing ([Dark and the long term](https://blog.darklang.com/dark-and-the-long-term/)).
  - 2024: the custom editor was poor and cut users off from their usual tools;
    backwards compatibility was very costly; users feared lock-in from the
    licence and the cloud-only runtime. They dropped the editor for a language
    server and a CLI ([status update](https://blog.darklang.com/an-overdue-status-update/)).
  - 2025: Dark Inc ran out of money. AI coding tools also made the structured
    editor pointless. The assets went to Darklang Inc, run by former
    employees, and the code was relicensed under Apache 2.0 ([goodbye post](https://blog.darklang.com/goodbye-dark-inc-welcome-darklang-inc/)).
- **Status:** the blog stops in June 2025, but the repo is very active (a
  dozen merges on 2026-11-16). See section 3 for what it does today.
- **Lesson:** owning the editor, language, runtime and hosting means reaching
  parity on all of them before anything is useful. Lock-in was what users
  objected to most.

### Wing (2022–2025)

- **Pitch:** one language for cloud infrastructure and application code,
  compiled to Terraform plus JavaScript, with a local simulator
  ([winglang.io](https://www.winglang.io/)).
- **Delivered:** the compiler, SDK, simulator and console, with over 100
  contributors. The company raised a $20M seed in 2023.
- **Outcome:** the company shut down in April 2025. The CEO's explanation was
  that companies don't see developer experience as business-critical
  ([shutdown post](https://docsite-omega.vercel.app/blog/2025/04/09/shutdown)).
  Wing is now community-maintained under MIT.
- **Lesson:** the technology worked and developers liked it, but it didn't
  solve a problem anyone would pay for. (That matters less for weft; see
  section 4.)

### Opa and Meteor

- **Opa:** one functional source generating client, server and database code
  ([README](https://github.com/MLstate/opalang)). It went quiet around 2016,
  with documented friction over its AGPL licence. No primary source says why
  it ended.
- **Meteor:** promised that client and server query the same synced database
  with the same API, plus optimistic updates on the client. The sync was built
  on MongoDB's replication log, where every server instance reads every write,
  and it didn't scale ([forum thread](https://forums.meteor.com/t/oplog-flooding-experience-thread/2258)).
  Teams fell back to plain API calls. The company moved to Apollo and sold
  Meteor in 2019. Meteor itself is still maintained.
- **Lesson:** the automatic part was tied to one database, and its mechanism
  didn't scale. Users kept only the parts they could replace.

## 2. Ongoing and academic projects

### Unison

Definitions are identified by their content, not their names, and the stated
goal is to turn any pool of machines into one programmable cluster
([1.0](https://unison-lang.org/unison-1-0)). Unison Cloud reached general
availability in 2024, bring-your-own-cloud in 2025, and the language reached
1.0 in November 2025. It's alive, with modest traction, after about ten years.
**Lesson:** a new language that owns distribution can survive, but slowly.

### Hydro (Berkeley, now co-led with AWS)

- **Vision:** the 2021 paper splits a program into four concerns: program
  semantics, availability, consistency and targets ([New Directions in Cloud
  Programming](https://arxiv.org/abs/2101.01159)). It's the closest statement
  of weft's goal.
- **Delivered:** a Rust library, not a language ([hydro.run](https://hydro.run/)).
  - Which machine a value lives on is part of its type, and every network send
    is explicit.
  - Stream types record ordering and retries.
  - Non-determinism has to be marked with `nondet!`.
  - Deterministic simulation is in preview.
- **Status:** still pre-1.0 (0.17 alpha, September 2026).
- **Lesson:** it shipped by embedding in an existing language and making
  placement explicit, not inferred.

### Multitier languages: Ur/Web, Links, ScalaLoci

One typed source for every tier of a web app. They solved type mismatches
between tiers, but stayed niche. A [2020 survey](https://scala-loci.github.io/publications/2020-csur/paper.pdf)
lists the reasons:
- they handle client–server apps only;
- they offer one implicit consistency level;
- they have no fault-tolerance abstractions;
- whole-program compilation rules out building large programs in pieces.

All three repos have seen little activity since 2023–24.

### Narrower takes that got traction

| Project | Scope it picked | Status |
|---|---|---|
| **Temporal** | Durable workflows through SDKs in existing languages. Workflow code must be deterministic so it can be replayed; "activities" are the escape hatch. | $300M Series D at $5B, February 2026 |
| **Convex** | A reactive TypeScript backend. Queries and mutations must be deterministic (randomness and time are fixed per call); "actions" are the escape hatch. | $57M Series B, August 2026 |
| **Encore** | A TypeScript or Go SDK. Infrastructure is declared as objects in code and provisioned in your own cloud account. | Active, about 12.7k GitHub stars |
| **Zero / Replicache** | Query-driven sync from Postgres to TypeScript clients | Replicache is in maintenance mode. Zero is explicitly not local-first and doesn't support offline writes ([when to use](https://zero.rocicorp.dev/docs/when-to-use)) |
| **Electric Clojure** | Macros split reactive UI code between client and server | Still alpha; commercial licence; offline not yet tackled |

### Infrastructure from code

A 2025 conference talk called the category a failure ([The New Stack](https://thenewstack.io/infrastructure-from-code-what-went-wrong/)).
The reasons given: the vendor chose the infrastructure, defaults didn't meet
compliance needs, outages depended on the vendor, platform engineers resisted,
and it impressed more in demos than in practice. Klotho and Wing stopped,
Shuttle pivoted, and Nitric and Encore continue.

## 3. Darklang today, under the hood

Read from the repo on 2026-11-17 (`refs/darklang`). The repo describes this
version, "dark-next", as not yet ready for production.

- **What it's for:** backends and CLIs, written in a functional, ML-style
  language and run by an interpreter written in F#.
- **Code lives in a database, not files.** Every function is immutable and
  identified by a hash of its content. An edit points a name at a new hash and
  keeps the old one. Version control (branches, commits, merges, rebases) is
  built in, stored as a log in a local SQLite file (`docs/internals.md`).
- **Live updates come from that design.** A running server follows edits by
  resolving names again, and work already in progress keeps the version it
  started with. This is how "no deploy step" works now (`docs/live.md`).
- **Effects are inferred, for permissions.** Static analysis over the code
  finds what each function might do: file reads and writes, HTTP, processes,
  environment, clock, randomness and more. `dark effects fn-name` shows the
  result and says when the analysis is incomplete. You approve access per
  immutable version, and editing the function invalidates the approval
  (`docs/effects-and-permissions.md`, `docs/effects.md`).
- **Recorded runs.** Each run is kept with a log of what it did to the world,
  and can be paused, resumed or forked by replaying that log. The scheduler
  runs many lightweight processes: reads run concurrently and writes keep
  their order (`docs/processes.md`).
- **Built for AI agents:** `AGENTS.md`, plus a doc of mistakes agents commonly
  make (`docs/agent-traps.md`).

### Darklang compared with weft

| Weft wants | Darklang |
|---|---|
| End-to-end: UI, server, storage | Backends and CLIs; no client |
| Offline clients that sync | One runtime; no sync between machines found |
| One program split across machines by a compiler | One interpreter runs everything |
| Distributed behaviour checked against a spec | Recorded runs show what happened; nothing checks them against a spec |
| Fast | Interpreted; compilation is planned |

Darklang removes **operational** friction by owning the runtime. Weft aims to
remove **distributed-systems** friction by compiling to many runtimes and
checking the result. For systems that are backends in one place, Darklang
would be worth using instead of building weft. For offline UIs plus servers,
it doesn't go there.

**Worth borrowing:**
- **Immutable, content-addressed code** as an answer to weft's evolution
  problem: when an old offline client replays commands, the server can know
  exactly which version of the code produced them.
- **Recorded runs you can replay and fork:** the debugging half of weft's
  simulator, for real runs as well as simulated ones.
- **Approval per code version** for weft's externs.
- **Docs written for agents** (`AGENTS.md`, `agent-traps.md`).

### Questions for Darklang's authors

1. Building it just for yourself, with no company, what would you cut first,
   and what was essential from day one?
2. Which part of owning the whole stack cost the most and mattered least?
3. What did Classic's datastores guarantee (transactions, consistency,
   schema)? Did users hit problems?
4. How did schema changes work when code went live on every edit? What made
   backwards compatibility so expensive, and does content-addressed code fix
   it?
5. Did recorded traces catch real bugs, or mostly help understanding? How far
   are they from deterministic replay?
6. Why are effects used for permissions rather than for deciding where code
   runs, or what is safe to retry?
7. Was a frontend or sync ever considered, and what ruled it out?
8. How well do agents write a language they've never seen, and what fixed the
   worst mistakes?
9. Weft treats the single-machine program as the spec and checks the
   distributed version against it. Where does that break?

## 4. Lessons for weft

### Recurring failure modes

1. **The parity trap.** Owning the whole system means building auth, packages,
   tooling, deployment and debugging before it's useful (Dark, Eve).
2. **Lock-in with no way out:** cloud-only runtimes, restrictive licences, one
   database, vendor-chosen infrastructure (Dark, Opa, Meteor, infrastructure
   from code).
3. **Custom tooling that cuts people off from their usual tools** (Dark's
   editor, Eve's environment). AI tools made this worse.
4. **Evolution costs:** compatibility guarantees slow every change (Dark).
5. **The automatic part stops scaling** (Meteor's sync).
6. **Narrow generality:** client–server only, one consistency level, no
   failure model (the multitier languages).
7. **No business that pays for developer experience** (Wing, Dark, Eve).

### What changes because weft is for personal use

Failure mode 7 doesn't apply, and a new language becomes easier to justify:
with one user, enjoying the language is a legitimate reason. Two risks get
more important:

- **Time to usefulness.** The parity trap applies to one person too. The goal
  is building the systems, not the tool. A practical guard: pick the next real
  app as the benchmark, and build only what it needs.
- **AI agents writing the language.** Agents have never seen weft, and if
  they're writing most of the code, that costs something every day. That
  argues for a small grammar close to known languages, compiler errors written
  as fix instructions, and documentation dense enough for an agent to load
  (Darklang's `AGENTS.md` is a model).

### What the projects that worked narrowed to

- **Embedded in an existing language,** not a new one (Temporal, Convex,
  Encore, Zero, Hydro, Electric, and JAX itself). Unison is the exception, and
  it took a decade.
- **A restricted, deterministic subset with an explicit escape hatch:**
  Temporal's workflows and activities, Convex's mutations and actions, Hydro's
  `nondet!`. This is weft's "restricted model with escape hatches", applied to
  one concern.
- **One concern and one target.**
- **Hard guarantees given up on purpose.** Zero dropped offline writes.
- **Placement stated explicitly before any attempt to infer it** (Hydro).

### The biggest feasibility risks

1. **"As if on one machine" can't hold for every invariant with offline
   writes.** Invariants such as uniqueness or "at most N" need coordination
   (the CALM theorem). Choosing weaker consistency changes what the program
   means, and weft must say so. The honest promise is: *the distributed app
   behaves like some one-machine run, or the compiler says which invariant it
   can't keep, and why.* The delete race in
   [examples/todo-lists](https://github.com/Pickleboyonline/weft/blob/main/examples/todo-lists/README.md) is a small instance.
2. **Scope multiplication:** a language, a compiler, client and server code,
   sync, storage, a simulator and several targets. Each of these broke at
   least one project above.
3. **Interop:** real apps need existing auth, payments and databases.
4. **Evolution:** old offline clients replaying commands against new server
   code.
5. **Agents writing an unfamiliar language** (above).

### A credible first scope

- One topology: one authoritative server (SQLite or Postgres) and browser
  clients.
- One sync model: clients apply commands immediately, queue them while
  offline, and the server replays them. Rejections come back as typed
  results.
- Placement and consistency written out explicitly; nothing inferred yet.
- The simulator first. It's the part nobody else offers, and it could even
  start as a testing tool for apps that already exist.
- Deferred: multiple clouds, a custom editor, automatic placement, and
  anything beyond client–server.

The open question this leaves is whether the first front end is the new
language or one embedded in an existing language. That's for the project
owner to decide; see [language.md](language.md), section 8.
