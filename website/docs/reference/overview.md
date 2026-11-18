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
  starting `// error:`, followed by what the compiler reports.
- **"What it compiles to"** notes say how a construct runs. They describe the
  design, not a promise about any particular platform.

## The whole language on one screen

```weft
/// A record is a nominal type: fields, then methods.
record Todo {
  key id:  Id(Todo) = new_id()
  title:   Text
  done:    Bool = False

  label = |t| if t.done { "Done: ${t.title}" } else { t.title }
}

todos = Table(Todo)                       // storage, provided by the platform

/// A command: a server function the client calls to change data.
add_todo = Command(|title: Text| -> Result(Id(Todo), [EmptyTitle]) {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  Ok(todos.insert(Todo { title: cleaned }))
})

/// A query: a server function the client calls to read data. It compiles to
/// one database query.
open_todos = Query(|| -> List(Todo) { todos.filter(|t| not t.done).sort_by(.title) })

component TodoList {
  state draft   = ""
  state problem = None

  add = || {
    problem = None
    let title = draft
    draft = ""
    add_todo(title).await.on_err(|err| problem = err)
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
3. **Effects are inferred, never written.** The compiler knows exactly what
   every function reads, writes and sends, and reports an error where an
   effect isn't allowed. Nothing in a name or type marks it. See
   [Effects](effects).
4. **What a function is depends on what it does and how it's used,** not on a
   keyword or a folder. A function a handler calls that writes tables is a
   command; a function a view reads is a query. See
   [Commands and queries](commands-and-queries).
5. **Only three things can change:** a `let mut` inside a function body, a
   component's `state` inside that component's functions, and tables.
   Everything else is immutable.
6. **Every value is data or a function.** There are no objects with identity,
   no references, and no null.

## Not in weft, by design

| Absent | Instead |
|---|---|
| Macros | Generators that write ordinary code (`weft generate`), and compiler-derived methods |
| Exceptions | `Result` and `?` everywhere, commands included; `crash` for impossible states |
| `null` | `Option(a)`: `None` or `Some(x)` |
| Classes and inheritance | Records with methods; `where` clauses for generic code |
| Implicit conversions | Explicit methods: `n.to_dec()`, `x.to_text()` |
| Shared mutable state | Tables, changed only through their write methods |
| Semicolons | One statement per line |

## Not yet designed

**Users and authorization** are an open decision in the
[language design](/design/language). Until that's settled, every client can
call every command.
