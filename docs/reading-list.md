# Reading list

Papers behind each part of the design in [language.md](language.md), grouped by
the question they answer. Every citation and link was checked on 2026-11-14,
or on 2026-11-18 for the sections on queries and database behavior.
Where open-source code exists, it's cloned in `refs/` (see
[refs/README.md](https://github.com/Pickleboyonline/weft/blob/main/refs/README.md)).

## Start here

If you read six things, read these. Together they cover the whole problem.

1. Hellerstein, Alvaro. [Keeping CALM: When Distributed Consistency Is Easy](https://doi.org/10.1145/3369736). CACM 2020. *Which logic needs coordination at all. The theory under the compiler's coordination choices.*
2. Cheung, Crooks, Hellerstein, Milano. [New Directions in Cloud Programming](https://arxiv.org/abs/2101.01159). CIDR 2021. *The Hydro manifesto: the closest statement of weft's goal by others.*
3. Weisenburger, Wirth, Salvaneschi. [A Survey of Multitier Programming](https://doi.org/10.1145/3397495). ACM Computing Surveys 2020. *Everything tried so far on "one program for every tier."*
4. Cheung, Arden, Madden, Myers. [Automatic Partitioning of Database Applications](https://doi.org/10.14778/2350229.2350262). VLDB 2012. *Pyxis: a compiler deciding what runs where, from profiling.*
5. Bornholt et al. [Using Lightweight Formal Methods to Validate a Key-Value Storage Node in Amazon S3](https://doi.org/10.1145/3477132.3483540). SOSP 2021. *Reference model, property-based testing and crash checking in production: weft's testing layers, built by hand.*
6. Frostig, Johnson, Leary. [Compiling Machine Learning Programs via High-Level Tracing](https://mlsys.org/Conferences/doc/2018/146.pdf). SysML 2018. *The original JAX paper.*

## One program for every tier (multitier programming)

- Chlipala. [Ur/Web: A Simple Model for Programming the Web](https://doi.org/10.1145/2676726.2677004). POPL 2015. *One typed program compiled to a C server, a JS client and SQL.* Code: `refs/urweb`.
- Cooper, Lindley, Wadler, Yallop. [Links: Web Programming Without Tiers](https://doi.org/10.1007/978-3-540-74792-5_12). FMCO 2006.
- Weisenburger, Köhler, Salvaneschi. [Distributed System Development with ScalaLoci](https://doi.org/10.1145/3276499). OOPSLA 2018. *Placement types: the type says which tier a value lives on.* Code: `refs/scala-loci`.
- Radanne, Vouillon, Balat. [Eliom: A Core ML Language for Tierless Web Programming](https://doi.org/10.1007/978-3-319-47958-3_20). APLAS 2016.
- Electric Clojure has no paper; see `refs/electric/docs/electric-protocol.md`.

## Deciding what runs where, and how data is laid out

- Cheung, Arden, Madden, Myers. *Pyxis*: see Start here.
- Yan, Cheung. [Generating Application-specific Data Layouts for In-memory Databases](https://doi.org/10.14778/3342263.3342630). VLDB 2019. *Chestnut: data layouts synthesized from an app's queries.*
- Cheung, Solar-Lezama, Madden. [Optimizing Database-Backed Applications with Query Synthesis](https://doi.org/10.1145/2491956.2462180). PLDI 2013. *Turning imperative loops over data into queries.*

## Infrastructure as compositions of primitives

The theory behind replacing "use a time-series database" with primitives.

- Idreos et al. [The Periodic Table of Data Structures](https://open.bu.edu/items/8a35af4e-090b-4630-add6-5d37bd9405de). IEEE Data Engineering Bulletin 2018. *Data structures as combinations of a few design choices. Start with this one.*
- Idreos et al. [The Data Calculator: Data Structure Design and Cost Synthesis from First Principles and Learned Cost Models](https://doi.org/10.1145/3183713.3199671). SIGMOD 2018.
- Athanassoulis et al. [Designing Access Methods: The RUM Conjecture](https://openproceedings.org/2016/conf/edbt/paper-12.pdf). EDBT 2016. *Read, update, memory: pick two. The tradeoff behind storage annotations.*
- Chatterjee, Jagadeesan, Qin, Idreos. [Cosine: A Cloud-Cost Optimized Self-Designing Key-Value Storage Engine](https://doi.org/10.14778/3485450.3485461). VLDB 2022. *Designing storage to a cloud-cost budget.*
- Chaudhuri, Narasayya. [An Efficient, Cost-Driven Index Selection Tool for Microsoft SQL Server](https://www.vldb.org/conf/1997/P146.PDF). VLDB 1997. *The classic index advisor.*
- Pavlo et al. [Self-Driving Database Management Systems](https://www.cidrdb.org/cidr2017/papers/p42-pavlo-cidr17.pdf). CIDR 2017. *Re-planning from production telemetry: the "no static shapes" problem.*
- McSherry, Murray, Isaacs, Isard. [Differential Dataflow](https://www.cidrdb.org/cidr2013/Papers/CIDR13_Paper111.pdf). CIDR 2013. *Caches, materialized views and stream processing as one idea: keeping derived data up to date.*

## Queries in the language, with a guarantee

How table reads written as ordinary code can always become efficient
database queries, never N+1 and never a silent fallback.

- Cooper. [The Script-Writer's Dream: How to Write Great SQL in Your Own Language, and Be Sure It Will Succeed](https://doi.org/10.1007/978-3-642-03793-1_3). DBPL 2009. *Links: a query fragment that always normalizes to one SQL query.*
- Cheney, Lindley, Wadler. [A Practical Theory of Language-Integrated Query](https://doi.org/10.1145/2500365.2500586). ICFP 2013. *The theory weft's query fragment follows. Start with this one.*
- Cheney, Lindley, Wadler. [Query Shredding: Efficient Relational Evaluation of Queries over Nested Multisets](https://doi.org/10.1145/2588555.2612186). SIGMOD 2014. *Nested results as one query per level of nesting, never one per row.*
- Giorgidze, Grust, Schreiber, Weijers. [Haskell Boards the Ferry: Database-Supported Program Execution for Haskell](https://doi.org/10.1007/978-3-642-24276-2_1). IFL 2010 (LNCS, 2011). *The same idea for nested Haskell comprehensions.*
- Budiu, Chajed, McSherry, Ryzhyk, Tannen. [DBSP: Automatic Incremental View Maintenance for Rich Query Languages](https://doi.org/10.14778/3587136.3587137). PVLDB 2023. *Any relational query can be kept up to date from changes: how views stay in sync.*
- Microsoft. [Breaking changes in EF Core 3.x](https://learn.microsoft.com/en-us/ef/core/what-is-new/ef-core-3.x/breaking-changes). *Why silent client-side fallback was removed from Entity Framework.*

## When the database doesn't do what the app assumes

- Bailis, Fekete, Franklin, Ghodsi, Hellerstein, Stoica. [Feral Concurrency Control: An Empirical Investigation of Modern Application Integrity](https://doi.org/10.1145/2723372.2737784). SIGMOD 2015. *Rails validations, such as uniqueness, broken under concurrency in real apps.*
- Warszawski, Bailis. [ACIDRain: Concurrency-Related Attacks on Database-Backed Web Applications](https://doi.org/10.1145/3035918.3064037). SIGMOD 2017. *Exploitable isolation bugs in real e-commerce platforms.*

## Choosing the weakest safe coordination

- Hellerstein, Alvaro. *Keeping CALM*: see Start here.
- Alvaro, Conway, Hellerstein, Marczak. [Consistency Analysis in Bloom: a CALM and Collected Approach](https://www.cidrdb.org/cidr2011/Papers/CIDR11_Paper35.pdf). CIDR 2011.
- Sivaramakrishnan, Kaki, Jagannathan. [Declarative Programming over Eventually Consistent Data Stores](https://doi.org/10.1145/2737924.2737981). PLDI 2015. *Quelea: picking a consistency level per operation from declared contracts.*
- Gotsman, Yang, Ferreira, Najafzadeh, Shapiro. ['Cause I'm Strong Enough: Reasoning about Consistency Choices in Distributed Systems](https://doi.org/10.1145/2837614.2837625). POPL 2016. *Proving which operations need coordination to keep an invariant.*
- Houshmand, Lesani. [Hamsaz: Replication Coordination Analysis and Synthesis](https://doi.org/10.1145/3290387). POPL 2019. *Synthesizing that coordination from a sequential object plus invariants.*

## Merging concurrent changes

- Shapiro, Preguiça, Baquero, Zawirski. [Conflict-free Replicated Data Types](https://doi.org/10.1007/978-3-642-24550-3_29). SSS 2011.
- Laddad, Power, Milano, Cheung, Hellerstein. [Katara: Synthesizing CRDTs with Verified Lifting](https://doi.org/10.1145/3563336). OOPSLA 2022. *A CRDT derived from a sequential data type, which could replace hand-written `merge` rules.* Code: `katara` (commented out in `refs/repos.txt`).
- Kleppmann, Wiggins, van Hardenberg, McGranaghan. [Local-First Software: You Own Your Data, in Spite of the Cloud](https://doi.org/10.1145/3359591.3359737). Onward! 2019. *The case for offline-capable clients.*

## Compilers for distributed programs

- Cheung, Crooks, Hellerstein, Milano. *New Directions in Cloud Programming*: see Start here. Code: `refs/hydro`.
- Chu et al. [Optimizing Distributed Protocols with Query Rewrites](https://doi.org/10.1145/3639257). SIGMOD 2024. *Rewriting protocols such as Paxos to scale automatically.*
- Xu et al. [GSPMD: General and Scalable Parallelization for ML Computation Graphs](https://arxiv.org/abs/2105.04663). arXiv 2021. *How JAX propagates a few sharding annotations to a whole program: the model for placement.* Code: `refs/jax`.

## What "correct" means: consistency models

The theory the oracle needs, so that "allowed to differ from the local run"
has a precise meaning.

- Herlihy, Wing. [Linearizability: A Correctness Condition for Concurrent Objects](https://doi.org/10.1145/78969.78972). ACM TOPLAS 1990.
- Adya. [Weak Consistency: A Generalized Theory and Optimistic Implementations for Distributed Transactions](https://dspace.mit.edu/handle/1721.1/149899). PhD thesis, MIT, 1999. *Isolation anomalies defined as cycles in a dependency graph. Elle builds on this.*
- Viotti, Vukolić. [Consistency in Non-Transactional Distributed Storage Systems](https://doi.org/10.1145/2926965). ACM Computing Surveys 2016. *A map of about 50 consistency models.*
- Abadi. [Consistency Tradeoffs in Modern Distributed Database System Design: CAP is Only Part of the Story](https://doi.org/10.1109/MC.2012.33). IEEE Computer 2012. *PACELC: even without partitions, you trade latency for consistency.*

## Testing and verification

How these compare, with figures: [testing-and-proof.md](testing-and-proof.md).

**Testing and simulation**

- Claessen, Hughes. [QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs](https://www.cs.tufts.edu/~nr/cs257/archive/john-hughes/quick.pdf). ICFP 2000. *Properties checked on random inputs: the start of property-based testing.*
- Wilson. [Testing Distributed Systems w/ Deterministic Simulation](https://thestrangeloop.com/2014/testing-distributed-systems-w-slash-deterministic-simulation.html). Strange Loop 2014 (talk). *How FoundationDB was tested. The origin of deterministic simulation testing.* Code: `refs/tigerbeetle` for a modern version.
- Kingsbury, Alvaro. [Elle: Inferring Isolation Anomalies from Experimental Observations](https://doi.org/10.14778/3430915.3430918). VLDB 2021. *Checking recorded histories by finding dependency cycles: the scalable way to run the oracle.* Code: `refs/maelstrom` uses Jepsen's checkers.
- Claessen et al. [Finding Race Conditions in Erlang with QuickCheck and PULSE](https://doi.org/10.1145/1596550.1596574). ICFP 2009. *Random parallel commands checked against a sequential model.*
- Zhou et al. [FoundationDB: A Distributed Unbundled Transactional Key Value Store](https://www.foundationdb.org/files/fdb-paper.pdf). SIGMOD 2021. *Simulation in production use: what it caught, and what it can't (performance, code outside the simulator).*

**Model checking**

- Jackson. [Alloy and the small scope hypothesis](https://people.csail.mit.edu/dnj/talks/zb2003/zb2003.noanim.pdf). ZB 2003 (talk slides). *Most bugs have small counterexamples, so bounded checking finds a lot.*
- Newcombe et al. [How Amazon Web Services Uses Formal Methods](https://lamport.azurewebsites.net/tla/formal-methods-amazon.pdf). CACM 2015 (authors' preprint). *TLA+ on 10 production systems; bugs needing 35-step traces.*
- Bornholt et al. *ShardStore at Amazon S3*: see Start here. *A reference model, property-based testing and model checking instead of full proof.*
- [P](https://github.com/p-org/P) and [Stateright](https://github.com/stateright/stateright): model checking inside a programming language (state machines; Rust actors). Both are listed in `refs/repos.txt` (Tier 2, not cloned).

**Proof**

- Wilcox et al. [Verdi: A Framework for Implementing and Formally Verifying Distributed Systems](https://doi.org/10.1145/2737924.2737958). PLDI 2015. *Write for a perfect network, apply verified transformers for faults: the formal version of weft's idea.* Code: `refs/verdi`.
- Woos et al. [Planning for Change in a Formal Verification of the Raft Consensus Protocol](https://homes.cs.washington.edu/~mernst/pubs/raft-proof-cpp2016.pdf). CPP 2016. *The full Raft safety proof: about 50,000 lines for about 530 lines of code.*
- Hawblitzel et al. [IronFleet: Proving Practical Distributed Systems Correct](https://doi.org/10.1145/2815400.2815428). SOSP 2015. *Safety and liveness, in about 3.7 person-years.*
- Klein et al. [seL4: Formal Verification of an OS Kernel](https://www.sigops.org/s/conferences/sosp/2009/papers/klein-sosp09.pdf). SOSP 2009. *The cost benchmark: about 20 person-years.*
- Fonseca et al. [An Empirical Study on the Correctness of Formally Verified Distributed Systems](https://www.cs.purdue.edu/homes/pfonseca/papers/eurosys2017-dsbugs.pdf). EuroSys 2017. *16 bugs in verified systems, none in the proven protocols: all at the edges.*
- [Bend 2](https://github.com/bendlang/bend) (2026). *Laws written by a person, proofs written by an AI, checked by a dependently typed checker.*

## The language: effects and runtime

- Leijen. [Koka: Programming with Row Polymorphic Effect Types](https://doi.org/10.4204/EPTCS.153.8). MSFP 2014. Code: `refs/koka`.
- Reinking, Xie, de Moura, Leijen. [Perceus: Garbage Free Reference Counting with Reuse](https://doi.org/10.1145/3453483.3454032). PLDI 2021. *How functional record updates become in-place mutation.*
- Roc has no paper; see `refs/roc/docs/langref/platforms.md` and `functions.md`.

## Choosing cloud resources

- Stoica, Shenker. [From Cloud Computing to Sky Computing](https://doi.org/10.1145/3458336.3465301). HotOS 2021.
- Yang et al. [SkyPilot: An Intercloud Broker for Sky Computing](https://www.usenix.org/conference/nsdi23/presentation/yang-zongheng). NSDI 2023. *An optimizer over cloud offerings: the model for cost budgets.*

## Background

- Kleppmann. [Designing Data-Intensive Applications](https://www.oreilly.com/library/view/designing-data-intensive/9781491903063/). O'Reilly 2017. *The standard overview of storage, replication, partitioning and streams. A second edition (Kleppmann and Riccomini) came out in 2026.*
