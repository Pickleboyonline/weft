---
title: About this reference
sidebar_label: About this reference
description: How the weft language reference is organized, and the rules that hold everywhere.
---

# About this reference

This is the complete specification of the weft language: every construct,
what it means, and what it rejects. The [Start](/docs/) pages explain *why*;
this section says *what*.

:::note[A design, not yet a language]
No compiler exists yet. This reference is the design that one will implement.
Where a rule says "is a compile error", that's what the compiler will report.
:::

## How to read it

- **Every heading is linkable.** Link to a rule by its page and heading, e.g.
  [Control flow: `if`](control-flow#if-and-else).
- **Failing code is marked.** A line that doesn't compile ends with a comment
  starting `# error:`, followed by what the compiler reports.
- **"What it compiles to"** notes say how a construct runs. They describe the
  design, not a promise about any particular platform.

## The whole language on one screen

```weft
## A record is a nominal type: fields, then methods.
record Todo {
  key id:  Id(Todo) = new_id!()
  title:   Text
  done:    Bool = False

  label = |t| if t.done { "Done: ${t.title}" } else { t.title }
}

todos = Table(Todo)                       # storage, provided by the platform

## A function that writes a table is a command when a client calls it.
add_todo! : Text => Id(Todo) ! <reject [EmptyTitle], ..>
add_todo! = |title| {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  todos.insert!(Todo { title: cleaned })
}

## A function that only reads is a query when a view uses it.
open_todos = || todos.filter(|t| not t.done).sort_by(.title)

component TodoList {
  state draft   = ""
  state problem = None

  add = || {
    problem = None
    add_todo!(draft) else |err| problem = err
    draft = ""
  }

  view = main {
    form(on_submit: add) { input(bind: draft) }
    ul {
      for t in open_todos() { li { t.label() } }
    }
  }
}
```

## Rules that hold everywhere

1. **One project is one module.** There are no `module` lines and no
   `import`s within a project. Packages are named in `weft.toml` and used
   qualified (`ui_kit.Button`). See [Projects](projects).
2. **Two kinds of syntax.** *Declarations* (`record`, `component`, `effect`,
   `handler`, and bindings `name = …`) and *expressions*. Configuration is
   never a phrase: it's named arguments or record fields whose values are
   ordinary expressions.
3. **Purity is the default.** A function whose name ends in `!` may change the
   world. Every other function can't. See [Effects](effects).
4. **What a function is depends on what it does and where it lives,** not on a
   keyword. A function that writes tables, in `commands/`, is a command. A
   read-only function in `queries/` is a query. See
   [Commands and queries](commands-and-queries).
5. **Only three things can change:** a `var` inside a function body, a
   component's `state` inside that component's functions, and tables, through
   `!` functions. Everything else is immutable.
6. **Every value is data or a function.** There are no objects with identity,
   no references, and no null.

## Not in weft, by design

| Absent | Instead |
|---|---|
| Macros | Generators that write ordinary code (`weft generate`), and compiler-derived methods |
| Exceptions | `reject` in commands, `Result` and `?` in other functions, `crash` for impossible states |
| `null` | `Option(a)`: `None` or `Some(x)` |
| Classes and inheritance | Records with methods; `where` clauses for generic code |
| Implicit conversions | Explicit methods: `n.to_dec()`, `x.to_text()` |
| Shared mutable state | Tables, changed only by `!` functions |
| Semicolons | One statement per line |

## Not yet designed

**Users and authorization** are an open decision in the
[language design](/design/language). Until that's settled, every client can
call every command.
