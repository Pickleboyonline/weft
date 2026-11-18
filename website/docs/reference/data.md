---
title: Data and tables
description: Storing records in tables, references between tables, and the table API.
---

# Data and tables

## Tables

A table is stored rows of one record type. It's declared as a value:

```weft
lists = Table(TodoList)
items = Table(Item, delete_with: { list: lists })
```

- The record type must have exactly one `key` field. That field is the
  table's primary key.
- A table lives in the server's database, unless its `scope` says otherwise.
  Only queries, commands, jobs and other server code can read or write it (see
  [Server-only code](commands-and-queries#server-only-code)).
- Tables are top-level bindings in `data/`, in the file of the record they
  store.
- Writing to a table is the only way stored data changes.

### Options

| Option | Meaning |
|---|---|
| `scope: Shared`, `Device` or `Memory` | Where the rows live. `Shared` (the default) is the app's data, in the server's database. `Device` rows stay on one client device, kept across reloads, never sent anywhere: drafts, preferences, caches. `Memory` rows last only while the client runs. Client code reads and writes `Device` and `Memory` tables directly; a `Shared` table is reached only through queries and commands. |
| `delete_with: { field: table }` | When a row in `table` is deleted, delete the rows here whose `field` refers to it. `field` must be an `Id` of that table's record type. |
| `unique: [.field, …]` | No two rows may have the same value for these fields. On such a table, `insert`, `update` and `update_where` return a `Result`, failing with `Duplicate(field)` instead of breaking the rule. |

## References

A field of type `Id(T)` refers to a row of a table of `T`s:

```weft
record Item {
  key id: Id(Item) = new_id()
  list:   Id(TodoList)
  title:  Text
}
```

- References aren't enforced when written: an `Id` is just a value, and the row
  it names may have been deleted on another device. Check with `has` or
  `get` when it matters, and return an `Err` if it's gone.
- `delete_with` keeps references from dangling when the referenced row is
  deleted.

## Reading

| Method | Returns |
|---|---|
| `table.get(id)` | `Option(row)` |
| `table.has(id)` | `Bool` |
| `table.rows()` | `List(row)`: every row, fetched into memory |
| `table.filter(pred)` | `List(row)` |
| `table.count()`, `table.count_if(pred)` | `Int` |
| `table.any(pred)`, `table.all(pred)` | `Bool` |
| `table.find(pred)` | `Option(row)` |
| `table.map(f)`, `table.sort_by(f)`, `table.group_by(f)` | as for [lists](stdlib#list) |

A table has the other [`List`](stdlib#list) methods too, as if it were the
list of its rows: `items.take(10)`. A table's rows have no order of their
own; only `sort_by` gives one. In a simulation, unsorted results come back in
a different order on each run, so code that depends on an order it never
asked for is caught.

## Queries on tables

Every read of a shared table compiles to a database query. weft guarantees
how: **a read compiles to one
query, or to one query per level of nesting in its result, never one per row.
If it can't, that's a compile error.** There's no silent fallback that
fetches rows and filters them in memory.

The guarantee comes from language-integrated query research (Cooper's Links;
Cheney, Lindley and Wadler; see [the reading list](/design/reading-list)): code
written in the *query fragment* below can always be rewritten into one query.

### What a query is

The compiler treats every expression built from tables and fragment
operations as one query, across bindings and the helper functions it calls:

```weft
all_lists = Query(|| -> List({ list: TodoList, open: Int }) {
  let open = items.filter(|i| not i.done)
  lists
    .map(|l| { list: l, open: open.count_if(|i| i.list == l.id) })
    .sort_by(.list.created)
})
```

`all_lists` is one query: lists joined with a grouped count of open items,
sorted. It isn't one query for the lists plus one per list, and it doesn't
fetch every open item.

A result with lists inside it, such as every list *with its items*, is one
query per level of nesting: one for the lists and one for all their items,
put together afterwards.

### The query fragment

The functions given to table methods, and the operations chained after them,
may use:

- the row, the function's parameters, and any value that doesn't depend on a
  row. Those are computed before the query runs and passed in, so
  `items.filter(|i| i.created < now() - 30d)` reads the clock once;
- field access, records, tags, `if` and `match`;
- `==`, `!=`, `<`, `<=`, `>`, `>=`, `and`, `or`, `not`, and arithmetic;
- the methods the [standard library](stdlib) marks as query methods, such as
  `trim`, `lower`, `contains`, `starts_with`, `with_default`, and `contains` on
  a list passed in (`ids.contains(i.id)`);
- other functions whose bodies are in the fragment. They're inlined;
- reads of other tables, which become joins and grouped aggregates:
  `lists.all(|l| items.any(|i| i.list == l.id))`.

They may not use `let mut`, loops or recursion, call a method with no query form,
or have any effect other than reading tables. Code outside the fragment is a
compile error that says how to fix it:

```weft
fancy_score = |i| {
  let mut score = 0
  while score < i.title.length() { score = score + 1 }
  score
}

high_scores = Query(|| -> List(Item) { items.filter(|i| fancy_score(i) > 3) })
// error: this can't run as a query: `fancy_score` uses a `while` loop.
//        To filter in memory, fetch the rows first: items.rows().filter(|i| fancy_score(i) > 3)
```

`rows()` is the only way to bring a whole table into ordinary code, so every
place that does it is visible in the source and in `weft explain`.

### One meaning everywhere

- **A query means exactly what the same code means on lists in memory.** Where
  a database differs, as with text comparison, number overflow or the sum of
  nothing, the compiler generates a query that matches weft's meaning. If a
  platform can't match it, using that operation on that platform is a compile
  error naming the platform.
- **Reads of `Device` and `Memory` tables** run on the client, with the same
  meaning and the same fragment rules.
- **`weft explain` shows the plan:** the generated SQL, the indexes it chose
  from your filters and sorts, and how many queries each view makes.

## Writing

Writes are allowed anywhere except in queries, views, checks and `expect`
(see [Effects](effects#where-effects-are-allowed)).

| Method | Does | Returns |
|---|---|---|
| `table.insert(row)` | adds a row | the row's `Id` |
| `table.update(id, f)` | replaces the row with `f(row)` | `{}`; does nothing if the row is gone |
| `table.delete(id)` | removes the row | `{}`; does nothing if the row is gone |
| `table.update_where(pred, f)` | updates every matching row | `{}` |
| `table.delete_where(pred)` | removes every matching row | `{}` |

```weft
rename_item : Id(Item), Text -> Result({}, [EmptyTitle])
rename_item = |id, title| {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  Ok(items.update(id, |i| { ..i, title: cleaned }))
}
```

Updating or deleting a row that no longer exists does nothing, because on
another device it may already be gone, and the person's intent ("rename this",
"delete this") no longer applies. A command that must know, checks first and
fails:

```weft
if not items.has(id) { return Err(NoSuchItem) }
```
