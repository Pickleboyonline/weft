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
# A comment runs from # to the end of the line.

## A doc comment starts with ##. It documents the declaration right below it,
## and is written in Markdown.
keeps = |show, i| match show {
  All    -> True
  Active -> not i.done
  Done   -> i.done
}
```

- Doc comments attach to the next declaration. `weft docs` renders them, and
  editors show them on hover.
- There are no block comments.

## Names

| Kind | Form | Examples |
|---|---|---|
| Values, functions, fields, state | `snake_case` | `draft`, `items_in`, `created` |
| Effectful functions | `snake_case!` | `add_item!`, `now!` |
| Types, tags, components | `PascalCase` | `Item`, `EmptyTitle`, `ItemRow` |
| Type variables | a single lowercase word, in signatures | `a`, `item`, `err` |

- **The `!` is part of the name.** A function's name ends in `!` exactly when
  it may change the world (see [Effects](effects#the-rule)). The compiler
  checks this both ways.
- **No shadowing.** A name can't be declared again where it's already in
  scope:

  ```weft
  title = "a"
  check = |title| title.trim()     # error: `title` already exists in this scope
  ```

- **`_`** names a value you don't use: `|_, i| i.done`.

## Keywords

Reserved everywhere; never usable as names:

```text
and  break  component  continue  crash  effect  else  expect  for  handler
if  in  match  not  or  record  reject  return  state  style  then  var
where  while  with
```

Contextual: keywords only in one position, ordinary names elsewhere.

| Word | Keyword where | Meaning |
|---|---|---|
| `key` | before a field in a `record` | the record's primary key |
| `view` | as a binding inside a `component` | the component's markup |
| `ctl`, `final` | before an operation in an `effect` | the operation's kind |

`True`, `False`, `None`, `Some`, `Ok` and `Err` are ordinary tags from the
standard library, not keywords.

## Literals

### Numbers

```weft
count  = 42
big    = 1_000_000        # underscores are ignored
price  = 19.99            # a Dec: exact decimal
ratio  = 0.5f             # the f suffix makes a Float
```

- A whole-number literal is an `Int`, `Dec` or `Float`, whichever the context
  needs. Without context it's an `Int`.
- A literal with a decimal point is a `Dec`, unless it ends in `f`.

### Durations, sizes and percentages

```weft
timeout = 30s
retention = 30d
latency = 5ms..=2s        # a Range(Duration)
budget = 64MB
drop = 10%                # the Dec 0.1
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
  `.`, `then`, `else` or a closing bracket:

  ```weft
  all_lists = || lists
    .sort_by(.created)
    .map(|l| { list: l, open: items.count_if(|i| i.list == l.id and not i.done) })
  ```

- **Braces hold either declarations or values.** Braces after `record`,
  `component`, `effect`, `handler` or `style`, and braces around a block of
  statements, hold one declaration or statement per line, with no commas.
  Braces that build a record value hold fields separated by commas. A trailing
  comma is allowed.
