---
title: Types
description: Built-in types, records, tag unions, collections, functions, type variables and inference.
---

# Types

weft is statically typed, and types are inferred: you write annotations only
where they're required (see [Annotations](#annotations)) or where they help a
reader.

## Built-in types

| Type | Values | Notes |
|---|---|---|
| `Int` | 64-bit signed whole numbers | overflow is a [runtime error](runtime-errors) |
| `Dec` | exact decimals, 18 digits after the point | for money and anything a person reads |
| `Float` | 64-bit IEEE floating point | for measurements; never for money |
| `Bool` | `True`, `False` | a tag union: `[False, True]` |
| `Text` | Unicode text | length counts characters, not bytes |
| `Bytes` | raw bytes | |
| `Time` | an instant, in UTC | |
| `Date` | a calendar date | |
| `Duration` | a length of time | literals: `30s`, `5min`, `2d` |
| `Size` | an amount of data | literals: `64MB` |
| `Id(t)` | the id of a row of record type `t` | opaque; see [Ids](#ids) |
| `{}` | the empty record | what a function returns when there's nothing to return |

## Records

A **nominal record** is declared with `record`. Two records with the same
fields but different names are different types.

```weft
record Item {
  key id:  Id(Item)     = new_id()
  list:    Id(TodoList)
  title:   Text
  done:    Bool = False
  created: Time = now()
}
```

- **Fields** are `name: Type`, optionally with a default after `=`.
- **`key`** marks the primary key. A record stored in a
  [`Table`](data#tables) must have exactly one.
- **Defaults** are evaluated each time a record is built without that field.
  A default may call an effectful function, such as `now()`; then building
  the record is effectful too.
- **Methods** are bindings inside the braces: see
  [Functions: methods](functions#methods). A line with `:` is a field; a line
  with `=` is a method.

Build, read and update records:

```weft
item = Item { list: id, title: "milk" }      // defaults fill id, done, created
title = item.title
done_item = { ..item, done: True }           // a copy with one field changed
renamed = Item { list: id, title }           // `title` is short for `title: title`
```

An **anonymous record** has no declaration. Its type is its fields:

```weft
row = { list: l, open: 3 }                   // type: { list: TodoList, open: Int }
```

## Tag unions

A **tag union** lists the shapes a value can take. Tag unions are
structural: two unions with the same tags are the same type.

```weft
Filter : [All, Active, Done]                 // a name for a union
Shape : [Circle(Dec), Rect({ w: Dec, h: Dec })]
```

- A tag can carry a payload: `Circle(2.0)`.
- Unions grow by inference. If a function can produce `EmptyTitle` or
  `NoSuchList`, its result type is `[EmptyTitle, NoSuchList]` without anyone
  declaring it.
- `Name : T` declares an *alias*: another name for a type. Aliases can't have
  methods; give a type methods by making it a `record`.

## Option and Result

```weft
Option(a)      : [None, Some(a)]
Result(ok, err) : [Ok(ok), Err(err)]
```

- There's no `null`. A value that may be missing is an `Option`:
  `lists.get(id)` returns `Option(TodoList)`.
- Anything that can fail returns a `Result`, commands included. A command
  that returns `Err` is also rolled back (see
  [Commands: failing](commands-and-queries#failing)).

## Collections

| Type | What it is |
|---|---|
| `List(a)` | ordered values |
| `Dict(k, v)` | values by key; `k` must have equality and hashing |
| `Set(a)` | unique values |
| `Range(a)` | the values between two bounds: `1..<5`, `5ms..=2s` |
| `Table(r)` | stored rows of record type `r`; see [Data](data) |

Collections are values: changing one gives a new collection and leaves the
original as it was.

## Ids

- `Id(Item)` is the id of an `Item` row, and can't be mixed up with an
  `Id(TodoList)`. A field of type `Id(TodoList)` is a reference to a row in a
  table of `TodoList`s.
- Ids are opaque. `new_id()` makes one, and their text form (`id.to_text()`,
  and in URLs) is a 26-character URL-safe token.
- An id says nothing about whether its row still exists. Check with
  `table.get(id)` or `table.has(id)`.

## Function types

```weft
keeps    : Filter, Item -> Bool
add_item : Id(TodoList), Text -> Result(Id(Item), [EmptyTitle, NoSuchList])
```

- A function type is its parameter types, `->`, and its result type.
- Types don't say what effects a function has, or whether it has any. The
  compiler infers that, and your editor shows it (see
  [Effects](effects#inferred-never-written)).
- A function taking no arguments has type `() -> T`.
- UI callbacks have the type `Action`, which is `() -> {}`, or `Action(a)`,
  which is `a -> {}`: see [UI: passing handlers](ui#passing-handlers-to-children).

## Type variables and `where`

Lowercase names in a signature are type variables: the function works for any
type in that position.

```weft
first_or : List(a), a -> a
first_or = |list, fallback| list.first().with_default(fallback)
```

A `where` clause says which methods a type variable must have:

```weft
total : List(a), a -> a where [a.plus : a, a -> a]
total = |values, zero| values.fold(zero, |sum, v| sum.plus(v))
```

There's no "implements" declaration: any type with those methods qualifies.
This is static dispatch, decided at compile time. See
[Functions: methods](functions#methods).

## Annotations

Annotations are optional, except:

1. **`Query(…)` and `Command(…)`** functions must declare their parameter
   types and their result type, which for a command that can fail is a
   `Result` listing its reasons. They are the API between clients and the
   server, and a changed signature is a changed contract.
2. **Externs** must declare their full type: weft can't see inside them.
3. **A `where` clause** is required when an annotated function calls a method
   on a type variable.

Anywhere else, the compiler infers types and your editor shows them on hover.

## Equality, ordering and text

Every record, tag union, list, dict, set and built-in type automatically has:

- `==` and `!=` (structural equality);
- `<`, `<=`, `>`, `>=` and `compare` (field by field, in declaration order;
  tags in declaration order);
- `to_text` (a readable form, e.g. `Item { title: "milk", … }`);
- `hash`, so it can be a `Dict` key or `Set` member;
- encoding, so it can be stored and sent between client and server.

Functions are the exception: they have none of these, which is why
[component state](ui#state) and table rows can't hold functions.

A record can replace any of these by defining the method itself.
