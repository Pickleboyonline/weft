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
- Any number of browsers share the same lists, and they can work offline.

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
    lists.weft               create_list!, rename_list!, delete_list!
    items.weft               add_item!, rename_item!, set_done!, delete_item!, clear_done!
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

There are no `command`, `query` or `check` keywords. A function in
`commands/` that writes is a command, a read-only function in `queries/` is a
query, and a `Bool` in `checks/` is a check.

What's missing on purpose:
- `deploy local`, which is generated;
- API routes, fetch calls, JSON, SQL, migrations, caching, retry logic,
  duplicate-request handling, loading states, and tests for any of those.

The compiler is supposed to produce all of them. A deployed project would also
have a generated `schema/` folder; nothing has been deployed, so there isn't
one.

## What to look at

### 1. `set_done!` instead of `toggle!`

The checkbox sends `set_done!(item.id, d)` with the checkbox's new value: what the person wants, not
"flip it". If your phone and laptop both tick the same box, the item ends up
done, which is what you both meant. With a `toggle!` it would flip twice and end
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
- **Delete, then add:** `add_item!` rejects with `NoSuchList` and the phone
  shows "That list was deleted."

Either way, no item is left without a list. That's the `no_orphans` check in
[checks/items.weft](checks/items.weft).

Across devices, a naive backend gets it wrong. It checks that the list exists
on the phone (where it did), then stores the item on the server without
checking again. The compiler must instead re-run `add_item!`'s check on the
server, ordered against any `delete_list!` of the same list. Nobody writes that
ordering; the compiler sees that `add_item!` *reads* `lists` before it writes,
and that `delete_list!` *deletes* from `lists`.

If a lowering got this wrong, the simulator would report something like:

```
$ weft simulate two_devices
check no_orphans FAILED  (seed 1729; shrunk from 600 steps to 2)
  browser 1: add_item!(Groceries, "milk")   list existed when checked on browser 1
  browser 2: delete_list!(Groceries)        reached the server first
  server:    stored "milk" for Groceries, which no longer exists
  allowed on one machine: "milk" deleted with the list, or add_item! rejects NoSuchList
  replay:    weft simulate two_devices --seed 1729
```

### 3. What each command needs across devices

The compiler works this out from the code. Nothing in the source asks for it.

| Command | What it does to the data | What the compiler adds |
|---|---|---|
| `create_list!`, `add_item!` (insert part) | Only adds rows | Nothing: inserts can arrive in any order and give the same result |
| `add_item!` (existence check) | Reads `lists` before writing | Re-check on the server, ordered with deletes of that list |
| `rename_list!`, `rename_item!`, `set_done!` | Overwrites a field | The server picks one order per row; every device ends up with the last write in that order |
| `delete_list!`, `delete_item!`, `clear_done!` | Removes rows | Ordered with writes to the same rows; late writes to deleted rows are dropped or rejected |
| All of them | | A request ID per click, so a retried request is applied once |

### 4. Queries that look slow but shouldn't be

`all_lists()` counts open items separately for every list. Written by hand
against a server, that's one query per list (the "N+1" problem). The compiler
should turn it into a single grouped count, or, since `deploy prod` replicates
both tables to the browser, compute it locally and update it as items change.

## What the compiler would produce

Illustrative output of the planned `weft explain` command:

```
$ weft explain prod
lists          Postgres table on server; replicated to every browser
items          Postgres table on server, index on (list, created); replicated to every browser
all_lists      computed in the browser from its copy; one grouped count, updated as items change
items_in       computed in the browser from its copy
create_list!   browser applies at once; server stores it            (insert only: no ordering)
add_item!      browser applies at once; server re-checks the list   (ordered with delete_list! per list)
set_done!      browser applies at once; server orders it per item
delete_list!   browser applies at once; server orders it per list
consistency    session: each browser sees its own changes immediately, others' after sync
```

```
$ weft run local                 # one process, in memory: the reference
$ weft simulate flaky_wifi       # 3 browsers, dropped messages, offline periods
$ weft deploy prod
```

## Gaps

Places where the design is still unsettled, found by writing this example.
Each is a question for the project owner, not a decision.

1. **One `problem` per component.** `ListPanel` shows rename errors and
   add-item errors through a single `problem` state, so a failed rename can
   replace the message from a failed add. Separate states work but repeat
   themselves. A per-form error helper, or rejections shown next to the form
   that caused them, would fit better.
2. **Users and authorization.** There are no users: everyone sees every list.
   This is an open decision in [docs/language.md](../../docs/language.md), and
   per-user lists are the natural next version of this example.

Settled by the reference since earlier versions of this example: the table
API, updating a missing row (a no-op), references between tables (`Id(T)` and
`delete_with`), ids in URLs (an opaque 26-character token), the not-found page
(`ui/pages/404.weft`), and navigating after a command that can reject (use
`then`, as the sidebar does after `create_list!`).
