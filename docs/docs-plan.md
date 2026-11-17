# Documentation plan

How weft's documentation should be structured, and which existing projects each
part is modeled on. Based on a survey of language docs done on 2026-11-17;
URLs are at the end. The site itself lives in [website/](../website/).

## Order of work

Write three top-level pages before any syntax documentation. They settle the
purpose, the promise and the decisions, and every later page depends on them.

1. **The big idea**, modeled on Unison's: one central idea, then its
   consequences in order, each set against the status quo. The idea: the
   single-machine program is the spec, and everything else is checked against
   it.
2. **The promise and its limits**, modeled on TigerBeetle's safety page:
   exactly what weft guarantees, what faults the simulator injects, and what
   happens when an invariant can't be kept offline.
3. **Why doesn't weft…?**, modeled on Roc's FAQ: each decision with its
   tradeoff, and firm "no"s where a decision is made. "Why a new language?"
   belongs here, with an answer that survives the history in
   [prior-art.md](prior-art.md).

## Site structure

| Section | Pages | Modeled on |
|---|---|---|
| **Start** | The big idea · The promise and its limits · Weft at a glance (syntax on one page) · Why doesn't weft…? | Unison, TigerBeetle, Gleam, Roc |
| **Tutorial** | One CRUD app grown step by step: the pure core → the client → effects → talking to the server → storage → deployment in its own file → consistency choices → testing in the simulator | Elm's order, Hydro's single growing example, Rust's project chapters, Svelte's exercises |
| **How-to guides** | Add a field or migration · write a user-defined effect · use an escape hatch · debug a failing simulator seed · deploy to a target | Diátaxis how-to, Gleam guides |
| **Concepts** ("About…" pages) | About effects · About placement and consistency · About the simulator | Diátaxis explanation, Koka's "Why Koka?", TigerBeetle's concepts |
| **Reference** | Language reference · deployment reference · CLI, simulator options, standard library | Rust Reference, Go spec, Roc langref |
| **Coming from…** | Weft for Elm users · for TypeScript + Postgres users · for Hydro users | Gleam's cheatsheets |
| **Effective weft** | Idioms, dated so it can't silently go stale | Effective Go |

## Rules for every page

**Code**
- Every code sample is compiled and tested, so the docs can't drift from the
  language (Zig). Until a compiler exists, samples are at least checked by the
  grammar tests in [editors/vscode](../editors/vscode/README.md).
- Code that's *supposed* to fail is marked as such: a rejected placement or an
  effect error is a teaching moment (Rust's Ferris icons).
- Inferred effects appear on hover in code samples (Koka).

**Writing**, adapted from Roc's [langref style guide](https://github.com/roc-lang/roc/blob/main/docs/langref-style-guide.md):
- Address the reader as "you".
- Define a term in italics where it's introduced.
- No compiler-internals vocabulary ("unification", "lowering") in user docs.
- Explain why, including the tradeoff. Say "by design" for deliberate
  omissions.
- Put the failing case next to the working case.
- Say who a caveat is for.
- Be definitive.
- Every runtime topic gets a "What it compiles to" section.
- **Never describe a planned feature as if it exists.** While nothing compiles,
  every page states that it describes a design.

**Structure**, from Diátaxis:
- Keep tutorials, how-to guides, reference and explanation apart. Explanation
  pages tend to absorb everything else; don't let them.
- Title explanation pages "About X", and keep instructions out of them.
- Teach the pure core before effects, then add effects with a diagram of what
  the runtime does with them (Elm).
- End each tutorial lesson with the whole program, then its parts, then
  exercises (Elm).
- Code that spans files shows the file tree, because weft splits logic
  (`src/`) from deployment (`deploy.weft`) (Svelte).

## Exemplars

| Project | What to take | Where |
|---|---|---|
| Elm | Architecture before types; installation late; whole program → parts → exercises | https://guide.elm-lang.org/ |
| Rust | Book / Reference / By Example split; linkable rule IDs; icons on failing code | https://doc.rust-lang.org/book/ · https://doc.rust-lang.org/reference/ |
| Gleam | ~70 tiny tour lessons; "uncomment to see the error"; cheatsheets per source language | https://tour.gleam.run/ · https://gleam.run/cheatsheets/gleam-for-elm-users/ |
| Roc | Style guide; "Why doesn't Roc…" FAQ with firm answers | https://www.roc-lang.org/faq |
| Koka | Effects introduced through a small table of labels; inferred effects on hover | https://koka-lang.github.io/koka/doc/book.html |
| Go | Spec teaching through legal and `// illegal:` examples; a separate idioms page | https://go.dev/ref/spec · https://go.dev/doc/effective_go |
| Svelte | Multi-file exercises with a file tree and a solve button | https://svelte.dev/tutorial |
| Diátaxis | Tutorials / how-to / reference / explanation | https://diataxis.fr/ |
| Unison | One "big idea" page, consequences in order, doubts addressed head-on | https://www.unison-lang.org/docs/the-big-idea/ |
| Hydro | One example grown step by step; simulation and safety as top-level pages | https://hydro.run/ |
| TigerBeetle | An explicit fault model: what the simulator injects | https://docs.tigerbeetle.com/concepts/safety/ |
| Zig | Reference samples compiled and tested in the main test suite | https://ziglang.org/documentation/master/ |
