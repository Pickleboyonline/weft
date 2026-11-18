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
- Where a table lives, how it's stored and how it's synced are decided by the
  deployment, not here (see [Deployment](deployment)).
- Tables are top-level bindings in `data/`, in the file of the record they
  store.
- Writing to a table is the only way stored data changes, and only `!`
  functions can do it.

### Options

| Option | Meaning |
|---|---|
| `delete_with: { field: table }` | When a row in `table` is deleted, delete the rows here whose `field` refers to it. `field` must be an `Id` of that table's record type. |
| `merge: { field: rule }` | How concurrent offline changes to a field combine. The default is to replay commands in order; see [Merging](#merging). |
| `unique: [.field, …]` | No two rows may have the same value for these fields. A write that would break this rejects with `Duplicate(field)`. |

## References

A field of type `Id(T)` refers to a row of a table of `T`s:

```weft
record Item {
  key id: Id(Item) = new_id!()
  list:   Id(TodoList)
  title:  Text
}
```

- References aren't enforced when written: an `Id` is just a value, and the row
  it names may have been deleted on another device. Check with `has` or `get`
  when it matters, and `reject` if it's gone.
- `delete_with` keeps references from dangling when the referenced row is
  deleted.

## Reading

Reads work in any function. They add `read` to its effects, without needing
`!`.

| Method | Returns |
|---|---|
| `table.get(id)` | `Option(row)` |
| `table.has(id)` | `Bool` |
| `table.filter(pred)` | `List(row)` |
| `table.count()`, `table.count_if(pred)` | `Int` |
| `table.any(pred)`, `table.all(pred)` | `Bool` |
| `table.find(pred)` | `Option(row)` |
| `table.sort_by(f)`, `table.map(f)`, `table.group_by(f)` | as for [lists](stdlib#list) |
| `for row in table { … }` | loops over every row |

A table also has every other [`List`](stdlib#list) method, as if it were the
list of its rows. The compiler turns each chain into a database query, an
index lookup, or a computation over a local copy, whichever is cheapest.

## Writing

Writes only work in `!` functions, and add `write` to their effects.

| Method | Does | Returns |
|---|---|---|
| `table.insert!(row)` | adds a row | the row's `Id` |
| `table.update!(id, f)` | replaces the row with `f(row)` | `{}`; does nothing if the row is gone |
| `table.delete!(id)` | removes the row | `{}`; does nothing if the row is gone |
| `table.update_where!(pred, f)` | updates every matching row | `{}` |
| `table.delete_where!(pred)` | removes every matching row | `{}` |

```weft
rename_item! : Id(Item), Text => {} ! <reject [EmptyTitle], ..>
rename_item! = |id, title| {
  cleaned = title.trim()
  if cleaned.is_empty() { reject EmptyTitle }
  items.update!(id, |i| { ..i, title: cleaned })
}
```

Updating or deleting a row that no longer exists does nothing, because on
another device it may already be gone, and the person's intent ("rename this",
"delete this") no longer applies. A command that must know, checks first and
rejects:

```weft
if not items.has(id) { reject NoSuchItem }
```

## Merging

By default, when a client was offline, its commands are replayed against the
server's data in the order they arrive. Every command then runs on one
machine's state, exactly as written, and the result is always explained by
some one-at-a-time order (see [The promise](/docs/promise)).

A field can instead merge concurrent changes with a rule:

```weft
items = Table(Item, merge: { done: Latest, tags: Union })
```

| Rule | Combines two changes by |
|---|---|
| `Latest` | keeping the later one, by the time each was made |
| `Union` | for `Set`s: keeping everything either side added |
| `Sum` | for counters: adding both changes |
| `Max`, `Min` | keeping the larger or smaller value |

A merged field no longer replays commands for that field, so a `check` that
depends on it is reported by the compiler as needing review.
