# todo-lists

A small CRUD app written in weft: named lists, each holding todo items.

**Nothing here runs.** There is no weft compiler yet. This is a design
artifact: it shows what a real weft project would look like, following
[docs/language.md](../../docs/language.md), and it exposes the places where
that proposal is still unspecified (see [Gaps](#gaps)). Command output in this
README is illustrative.

## The app

- A sidebar of lists, each with its count of open items. You can create,
  rename and delete lists.
- A panel for the selected list. You can add, rename, check off and delete
  items, filter by All / Active / Done, and clear the done ones.
- Any number of browsers share the same lists, and they can work offline.

## Files

| File | What's in it |
|---|---|
| [src/data.weft](src/data.weft) | Two records and two tables: `lists` and `items` |
| [src/logic.weft](src/logic.weft) | Commands (every write) and views (every read the client uses) |
| [src/app.weft](src/app.weft) | The browser client: model, messages, `update`, markup |
| [src/checks.weft](src/checks.weft) | Invariants. These are the project's only tests. |
| [deploy.weft](deploy.weft) | Where it runs: `local`, `prod`, and two simulations |

What's missing on purpose: API routes, fetch calls, JSON, SQL, migrations,
caching, retry logic, duplicate-request handling, loading states, and tests for
any of those. The compiler is supposed to produce all of them.

## What to look at

### 1. `set_done!` instead of `toggle!`

The checkbox sends `SetDone(id, not i.done)`: the value the person wants, not
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
[src/checks.weft](src/checks.weft).

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

### 4. Views that look slow but shouldn't be

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

Things this example had to assume that [docs/language.md](../../docs/language.md)
doesn't define yet. Each is a design question for the project owner, not a
decision.

1. **Modules across files.** Every file says `module todo_lists`, treating a
   directory as one module (like a Go package). There's no import syntax yet.
2. **Table API.** The example uses `lists.has(id)` and `lists.get(id)`
   returning `[Missing, Found(TodoList)]`. Neither is specified.
3. **Updating or deleting a missing row.** `set_done!` on an item that was just
   deleted elsewhere: a no-op, or a rejection? The example assumes a no-op,
   which is why `set_done!` has no `reject`.
4. **References between tables.** `Item.list` is a plain `Id`, so the cascade
   in `delete_list!` and the `no_orphans` check are written by hand. A
   reference type (`list: Ref(TodoList)` with "delete with parent") would let
   the compiler know the relationship directly.
5. **Helpers inside `client`.** `retext`, `save`, `list_panel`, `list_name`,
   `item_title` and `edit_box` are pure functions defined inside the client
   block. language.md only shows top-level helpers.
6. **Users and authorization.** There are no users: everyone sees every list.
   This is open decision 7 in language.md, and per-user lists are the natural
   next version of this example.
7. **Routing.** The selected list lives in the model, not the URL.
