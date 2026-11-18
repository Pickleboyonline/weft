# Testing and proof

How weft's checks could be verified: by example tests, property-based
testing, deterministic simulation, model checking or proof. The figures below
were checked against primary sources on 2026-11-18; the sources are in
[reading-list.md](reading-list.md), under "Testing and verification".

## The short version

A property-based test, a simulation check and a proven law can all state the
same thing: "no item is ever left without a list". What differs is **how the
statement is checked**, and so what "it passed" tells you.

- Testing, simulation and model checking produce **counterexamples**. They can
  show a bug exists; they can't show none do.
- A proof produces **a guarantee**, but only about the model it reasons over.
  It's only as good as the specification and the unproven parts around it.

## The spectrum

| Approach | What it checks | "It passed" means | A failure gives you | Cost |
|---|---|---|---|---|
| Example tests | inputs you picked | those cases work | a failing case | cheap; limited to what you thought of |
| Property-based testing (QuickCheck, Hypothesis) | a property over random inputs | no counterexample among the inputs tried | a counterexample, shrunk to the smallest | cheap; scales with compute |
| Deterministic simulation (FoundationDB, TigerBeetle) | the real code in a simulated world: network, disk, crashes, many clients | no failure in the runs tried | a seed that replays the failure exactly | moderate: the simulated world is built once |
| Model checking (TLA+, P, Stateright) | every behavior up to a size bound | nothing fails within the bound | a counterexample trace | a model or harness; can blow up with size |
| Proof (Lean, Coq, Dafny; Bend 2's laws) | all inputs and behaviors | true for everything, if the spec and trusted parts are right | a proof that won't go through: a bug, or just a hard proof | expensive human (or AI) effort |

## The evidence

### Proofs are strong and costly

| System | Code | Proof | Effort | What was proven |
|---|---|---|---|---|
| Verdi's Raft (Coq) | about 530 lines | about 50,000 lines | not stated | safety only; liveness left as future work |
| IronFleet (Dafny) | two systems | 3.6 lines of annotation per line of code | about 3.7 person-years | safety and liveness |
| seL4 kernel (Isabelle) | 8,700 lines of C | about 200,000 lines | about 20 person-years | functional correctness |

### Proven systems still had bugs, at the edges

Fonseca et al. (EuroSys 2017) spent eight months looking for bugs in three
verified distributed systems: IronFleet, Verdi and Chapar. They found 16, and
**none were in the proven protocols**. All were at the boundary between
verified and unverified parts:

- the unverified glue to the real network and disk (11; most crashed or hung
  servers);
- the specification itself (2);
- the verification tools (4).

(The categories sum to 17, not 16; the paper doesn't explain the gap.)

### The middle ground works in industry

- **AWS and TLA+** (Newcombe et al.): the TLC model checker on 10 large
  systems across 7 teams. Engineers were productive in 2–3 weeks. It found bugs
  in S3, DynamoDB and EBS; some DynamoDB bugs needed 35-step traces, far longer
  than anyone would write by hand. One liveness bug was missed because
  liveness wasn't checked.
- **S3's ShardStore** (Bornholt et al., SOSP 2021):
  - an executable reference model about 1% the size of the real code;
  - property-based tests that the code matches the model, including across
    crashes;
  - model checking for concurrency.

  This prevented 16 issues from reaching production, without aiming for full
  proof. The test harnesses were about 12% of the code base.

### Simulation works at scale

- FoundationDB runs tens of thousands of simulations every night.
- Apple's CloudKit ran it for over half a million disk-years without a data
  corruption event.
- Its stated blind spots are performance problems, and code outside its
  simulated framework.

### Why small bounds catch so much

Daniel Jackson's *small scope hypothesis*: most bugs have small
counterexamples. Checking every case up to a small size finds a surprising
share of them.

## How Bend 2's laws work

Bend 2 (2026) is built around proof as the way to trust AI-written code:

- **A person writes the claims** as laws in `LAWS.bend`:
  `law add_zero: for x: Nat {Nat.add(x, 0n) == x : Nat}`.
- **The AI writes the proofs** in `PROOF.bend`, as ordinary functions. A law
  is a type and a proof is a function of that type (dependent types).
  Pattern matching does case analysis, recursion does induction, and there's
  no automatic proof search.
- **`bend PROOF.bend` is the gate.** It reports either that all proofs check
  or that some fail. `--verdict` rechecks with a small kernel that is itself
  proven in Lean.
- **Bend's README lists edges of the same kind Fonseca found:**
  - the main checker isn't proven and may have bugs;
  - the parser isn't proven;
  - floating point is assumed, not proven;
  - compiled backends can differ from the proven semantics.

## What this means for weft

### Two different questions

Weft's correctness splits into two questions, and they suit different
methods.

1. **Is the app's logic right on one machine?** For example: does every
   command keep `no_orphans` true? This is sequential reasoning about
   functions over one snapshot of the tables, where Bend-style laws are cheap.
   Proving that `add_item` preserves `no_orphans` is an induction over one function, not a
   Raft-sized proof.
2. **Does the distributed version behave like the one-machine version?** This
   is a refinement property: the expensive kind Verdi and IronFleet proved.
   It's the compiler's job, not the app author's. Verdi's design shows how it
   could eventually be proven once per protocol template and reused by every
   app. Until then, the simulator checks it for each app.

### One check, three strengths

Rather than choosing one method, weft could let the same check be backed at
increasing strength:

| Strength | How | Catches | Cost |
|---|---|---|---|
| **Simulated** (default) | Random runs of the whole system against the one-machine program | Distribution bugs, platform and extern behavior, with a replayable seed | compute |
| **Exhausted** | Every behavior of a small configuration, e.g. 2 clients doing 3 actions each | Everything within the bound. The delete race is a 2-step counterexample. | compute; limited by state explosion |
| **Proven** | Each command shown to preserve the check on one machine; the person writes the check, an AI writes the proof | Logic errors, for all inputs | proof effort, and proofs to maintain as code changes |

### Simulation never goes away

The Fonseca study is a warning aimed squarely at weft. Its edges are the
compiler, the platforms and the externs, and that's exactly where proven
systems failed. A proof of the app's logic says nothing about them; the
simulator exercises them together, on every run.

### Open questions

- **Syntax for proofs.** Exhausting is designed: it's a setting on a
  simulation (`Simulation { exhaust: { actions_per_client: 3 } }`; see the
  reference's "Checks and tests" page). How a check is *proven* isn't: perhaps
  a separate file of proofs beside `checks/`, as Bend keeps laws and proofs in
  separate files.
- **Who writes proofs.** Bend's split (a person writes the law, an AI writes
  the proof) fits weft's agent-first workflow. It needs a proof language small
  enough for agents to write reliably.
- **Liveness.** Simulation and bounded checking can only check "eventually"
  within a step limit ("every screen refreshed within 50 steps after the network heals"), and
  the AWS team missed a bug by not checking liveness at all. Weft's
  convergence check is a liveness property, so it needs an explicit bound.
