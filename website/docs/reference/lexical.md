---
title: Lexical structure
description: Source files, comments, names, keywords, literals and line breaks.
---

# Lexical structure

## Source files

- Source files are UTF-8 text with the extension `.weft`.
- Where a file lives decides what it's for (see [Projects](projects)), but
  every file uses the same syntax.

## Comments

```weft
//! A file doc comment: it documents the whole file, and goes before anything
//! else in it.

// A line comment runs from // to the end of the line.

/// A doc comment documents the declaration right below it, and is written in
/// Markdown.
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done   /* a block comment can sit inside a line */
  Done   -> i.done
}

/* A block comment can span lines,
   /* and nests, so commenting out code that has one still works. */
*/
```

| Form | What it is |
|---|---|
| `// …` | line comment |
| `/// …` | doc comment for the next declaration |
| `//! …` | doc comment for the file it's in |
| `/* … */` | block comment; nests |

- Doc comments are Markdown. `weft docs` renders them, and editors show them
  on hover. A `///` that isn't followed by a declaration is a compile error.
- `//!` comments must come before any declaration in the file.
- Inside text (`"…"`), `//` and `/*` are just characters.

## Names

| Kind | Form | Examples |
|---|---|---|
| Values, functions, fields, state | `snake_case` | `draft`, `add_item`, `created` |
| Types, tags, components | `PascalCase` | `Item`, `EmptyTitle`, `ItemRow` |
| Type variables | a single lowercase word, in signatures | `a`, `item`, `err` |

- **Names don't say what a function does.** There's no marker for effects
  in names or types; the compiler infers them (see [Effects](effects)).
- **New local names start with `let`.** `let x = …` binds (and may shadow
  an existing `x`); `x = …` in a block changes a `let mut` or a `state`. See
  [Bindings](bindings).

- **`_`** names a value you don't use: `|_, i| i.done`.

## Keywords

Reserved everywhere; never usable as names:

```text
and  await  break  component  continue  crash  defer  effect  else  expect
for  handler  if  in  let  match  mut  not  or  record  return  state  style
where  while  with
```

Contextual: keywords only in one position, ordinary names elsewhere.

| Word | Keyword where | Meaning |
|---|---|---|
| `key` | before a field in a `record` | the record's primary key |
| `view` | as a binding inside a `component` | the component's markup |
| `task` | at the start of a line in a `component` or stateful function, before `{` or `(` | work tied to the component's life; see [UI](ui#tasks) |
| `ctl`, `final` | before an operation in an `effect` | the operation's kind |

`True`, `False`, `None`, `Some`, `Ok` and `Err` are ordinary tags from the
standard library, not keywords.

## Literals

### Numbers

```weft
count  = 42
big    = 1_000_000        // underscores are ignored
price  = 19.99            // a Dec: exact decimal
ratio  = 0.5f             // the f suffix makes a Float
```

- A whole-number literal is an `Int`, `Dec` or `Float`, whichever the context
  needs. Without context it's an `Int`.
- A literal with a decimal point is a `Dec`, unless it ends in `f`.

### Durations, sizes and percentages

```weft
timeout = 30s
retention = 30d
latency = 5ms..=2s        // a Range(Duration)
budget = 64MB
drop = 10%                // the Dec 0.1
```

| Suffix | Type | Units |
|---|---|---|
| `ns` `us` `ms` `s` `min` `h` `d` `w` | `Duration` | nanoseconds to weeks (`min` is minutes; there's no `m`) |
| `B` `KB` `MB` `GB` `TB` | `Size` | bytes, decimal multiples |
| `%` | `Dec` | `10%` is `0.1` |

A suffix must touch the number: `10%` is a percentage, and `10 % 3` is the
remainder operator.

### Text

```weft
greeting = "Hello, ${user.name}!"
escaped  = "Tab:\t quote:\" backslash:\\ newline:\n emoji:\u{1F600}"
letter   = """
  Dear ${name},

  Your list "${list.name}" was shared with you.
  """
```

- `${expr}` inserts any value that has a `to_text` method.
- Escapes: `\n`, `\t`, `\r`, `\"`, `\\`, `\$`, and `\u{…}` for any Unicode code
  point.
- `"""` starts a multi-line text. The indentation of the closing `"""` is
  removed from every line.

### Lists, records and tags

```weft
names  = ["milk", "eggs"]
point  = { x: 1, y: 2 }
status = Some(3)
```

## Lines and blocks

- **One statement per line.** There are no semicolons.
- **A statement continues onto the next line** when the line ends with an open
  bracket, a comma or a binary operator, or when the next line starts with
  `.` or `else`:

  ```weft
  items_in = |list, show| items
    .filter(|i| i.list == list and keeps(show, i))
    .sort_by(.created)
  ```

- **Braces hold either declarations or values.** Braces after `record`,
  `component`, `effect`, `handler` or `style`, and braces around a block of
  statements, hold one declaration or statement per line, with no commas.
  Braces that build a record value hold fields separated by commas. A trailing
  comma is allowed.
