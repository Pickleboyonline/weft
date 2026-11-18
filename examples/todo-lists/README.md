# todo-lists

A small CRUD app written in weft: named lists, each holding todo items.

**Nothing here runs.** There is no weft compiler yet. This is a design
artifact: it shows what a real weft project would look like, following the
language reference on the documentation site (`website/docs/reference/`), and
it exposes the places where the design still has gaps (see [Gaps](#gaps)).
Command output in this README is illustrative.

## The app

- A sidebar of lists, each with its count of open items. You can create,
  rename and delete lists.
- A panel for the selected list. You can add, rename, check off and delete
  items, filter by All / Active / Done, and clear the done ones.
- Any number of browsers share the same lists, stored on one server.

## Files

The project follows weft's conventional layout (reference: Projects), so where
each thing lives is predictable:

```text
todo-lists/
  weft.toml                  the project's name
  data/
    todo_list.weft           record TodoList, table lists
    item.weft                record Item, table items (deleted with their list)
    filter.weft              Filter : [All, Active, Done]
  commands/
    lists.weft               create_list, rename_list, delete_list
    items.weft               add_item, rename_item, set_done, delete_item, clear_done
  queries/
    lists.weft               all_lists, find_list
    items.weft               keeps, items_in, and expects for keeps
  ui/
    App.weft                 the root component: a sidebar beside the current page
    pages/
      index.weft             /            Home
      lists/[id].weft        /lists/:id   ListPage
      404.weft               anything else
    components/
      Sidebar.weft           every list, as links, and a new-list form
      ListPanel.weft         one list: rename, add items, filter, delete
      ItemRow.weft           one item: check off, edit inline, delete
    styles/
      global.css
  checks/
    lists.weft               names_not_blank
    items.weft               titles_not_blank, no_orphans
  deploy/
    prod.weft                a Postgres server and any number of browsers
  simulate/
    two_devices.weft         random runs on a slow network
    flaky_wifi.weft          random runs on a bad network
    small.weft               every order of 3 actions for 2 browsers
```

There are no `command`, `query` or `check` keywords. Remote functions are
values: `Command(…)` in `commands/`, `Query(…)` in `queries/`, and invariants
are `Check(…)` values in `checks/`. The folders are a convention.

What's missing on purpose:
- `deploy local`, which is generated;
- API routes, fetch calls, JSON, SQL, migrations, caching, retry logic,
  duplicate-request handling, loading states, and tests for any of those.

The compiler is supposed to produce all of them. A deployed project would also
have a generated `schema/` folder; nothing has been deployed, so there isn't
one.

## What to look at

### 1. `set_done` instead of `toggle`

The checkbox sends `set_done(item.id, d)` with the checkbox's new value: what the person wants, not
"flip it". If your phone and laptop both tick the same box, the item ends up
done, which is what you both meant. With a `toggle` it would flip twice and end
up not done. That's correct (it's what one machine would do with two toggles)
but surprising.

The compiler guarantees that the stored data always matches *some*
one-at-a-time order of everyone's clicks. It can't guess what people meant;
choosing the command that says what they meant is up to the programmer.

### 2. The delete race

This is the bug weft exists to catch. Two devices, one list:

| Time | Phone | Laptop | Server |
|---|---|---|---|
| 1 | Adds "milk" to Groceries | | |
| 2 | | Deletes Groceries | |
| 3 | | | Laptop's delete arrives first: list and its items removed |
| 4 | | | Phone's add arrives: stores "milk" in a list that no longer exists |

On one machine this can't happen. The two commands run one after the other, in
either order:

- **Add, then delete:** "milk" is deleted along with the list.
- **Delete, then add:** `add_item` fails with `NoSuchList` and the phone
  shows "That list was deleted."

Either way, no item is left without a list. That's the `no_orphans` check in
[checks/items.weft](checks/items.weft).

A naive backend gets it wrong even with one database. Postgres defaults to
Read Committed isolation, where `add_item`'s existence check can see the list
while the laptop's delete is still uncommitted. Both commit, and "milk" is
left in a list that no longer exists. weft promises that each command is
atomic and runs as if alone, so the compiler must lower `add_item` to
something that can't do this: a Serializable transaction retried on
conflict, or an equivalent it can show is safe, such as a foreign key or a
lock on the list row. Nobody writes that; the compiler sees that `add_item`
*reads* `lists` before it writes, and that `delete_list` *deletes* from
`lists`.

If a lowering got this wrong, the simulator would report something like:

```
$ weft simulate two_devices
check no_orphans FAILED (seed 1729; shrunk from 600 steps to 2)
  browser 1: add_item(Groceries, "milk")   checked that Groceries exists
  browser 2: delete_list(Groceries)        committed before add_item did
  server:    stored "milk" for Groceries, which no longer exists
  allowed on one machine: "milk" deleted with the list, or add_item fails with NoSuchList
  replay:    weft simulate two_devices --seed 1729
```

### 3. What the compiler adds to every command

Nothing in the source asks for any of this:

| Added | Why |
|---|---|
| An endpoint, a typed client call, and argument validation | Client and server can't disagree on types, and bad input never reaches the command |
| A request id, recorded in the same atomic run as the command's writes | A retried request (a double click, a dropped response) is applied once |
| An atomic lowering: by default a Serializable transaction, retried on conflict | `add_item` reads `lists` and `delete_list` deletes from it, so they must not interleave. Cheaper forms are used only where the compiler can show they're equivalent |
| A refresh of the queries that read what it wrote | After `add_item` writes `items`, every `items_in` and `all_lists` on screen updates, with nothing to invalidate by hand |

### 4. Queries that look slow but shouldn't be

`all_lists()` counts open items separately for every list. Written by hand
against a server with an ORM, that's one query per list (the "N+1" problem).
In weft it can't be: a table read is a language-integrated query, so the whole
expression compiles to one query, a join with a grouped count. Code the
compiler can't turn into one query is a compile error, not a slow fallback.

## What the compiler would produce

Illustrative output of the planned `weft explain` command:

```
$ weft explain prod
lists         Postgres table
items         Postgres table, index on (list, created)
all_lists     Query: 1 SQL query (lists left join items, grouped count); refreshed by writes to lists, items
items_in      Query: 1 SQL query on items; refreshed by writes to items
find_list     Query: 1 SQL query, by primary key; refreshed by writes to lists
create_list   Command: one INSERT plus its request id, in one transaction
add_item      Command: Serializable transaction, retried on conflict (reads lists, which delete_list deletes from)
set_done      Command: one UPDATE plus its request id, in one transaction
delete_list   Command: one DELETE (items deleted with it), plus its request id, in one transaction
```

```
$ weft run local                 # one process, in memory: the reference
$ weft simulate flaky_wifi       # 3 browsers, dropped messages, periods without the server
$ weft deploy prod
```

## Gaps

Places where the design is still unsettled, found by writing this example.
Each is a question for the project owner, not a decision.

1. **One `problem` per component.** `ListPanel` shows rename errors and
   add-item errors through a single `problem` state, so a failed rename can
   replace the message from a failed add. Separate states work but repeat
   themselves. A per-form error helper, or failures shown next to the form
   that caused them, would fit better.
2. **Users and authorization.** There are no users: everyone sees every list,
   and anyone can call every command. This is an open decision in
   [docs/language.md](../../docs/language.md), and per-user lists are the
   natural next version of this example.
3. **Changes wait for the server.** Adding an item shows it once the server
   confirms and `items_in` refreshes. Showing it at once (an optimistic
   update) belongs to the client cache design, which is still open.
4. **No offline use.** The browsers need the server. Offline writes, and
   keeping copies of `lists` and `items` in each browser, are part of the
   distribution design, deliberately left open.

Settled by the reference since earlier versions of this example: the table
API, updating a missing row (a no-op), references between tables (`Id(T)` and
`delete_with`), ids in URLs (an opaque 26-character token), the not-found page
(`ui/pages/404.weft`), navigating after a command that can fail (`.await`
its result and `match` it, as the sidebar does after `create_list`), effects (inferred, never
written), failures (commands return `Err`), checks (`Check` values), and
remote functions (`Query` and `Command` values).
