//! `weft explain <code>`: a longer explanation of each diagnostic, with an
//! example and the fix, like `rustc --explain`.

pub struct Entry {
    pub code: &'static str,
    pub title: &'static str,
    pub text: &'static str,
}

pub fn lookup(code: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.code == code)
}

pub const ENTRIES: &[Entry] = &[
    Entry {
        code: "syntax",
        title: "The code doesn't parse",
        text: "The parser couldn't read this as weft. Common causes: a missing closing
bracket, an `if` branch without braces, or an operator at the start of a line
(put it at the end of the previous line: a line break ends a statement
wherever the statement could end).

    let total = price      // ends here
      * quantity           // so this line starts with `*`

    let total = price *    // fix: the operator at the end
      quantity",
    },
    Entry {
        code: "chained-comparison",
        title: "Comparisons can't be chained",
        text: "`a < b < c` would compare a Bool with `c`. Write each comparison out.

    if 0 < n < 10 { … }            // error
    if 0 < n and n < 10 { … }      // fix",
    },
    Entry {
        code: "duplicate",
        title: "A top-level name is defined twice",
        text: "A project has one namespace, so each top-level name is defined once.
Rename one of them. Inside a block, use `let`, which may shadow.",
    },
    Entry {
        code: "unknown-name",
        title: "A name isn't defined",
        text: "Nothing in scope has this name. Inside a block, a new name starts with
`let`; plain `name = value` there is an assignment to something that already
exists.

    cleaned = title.trim()         // error: nothing called `cleaned` exists yet
    let cleaned = title.trim()     // fix",
    },
    Entry {
        code: "immutable",
        title: "Assigning a binding that can't change",
        text: "`let` bindings and parameters can't change. To make a new binding with
the same name, use `let` again (shadowing); to have a variable, declare it with
`let mut`.

    let cleaned = title.trim()
    cleaned = cleaned.lower()          // error
    let cleaned = cleaned.lower()      // fix: a new binding
    let mut count = 0                  // or a variable
    count = count + 1",
    },
    Entry {
        code: "var-escape",
        title: "A nested function assigns an outer variable",
        text: "Only the function that declared a `let mut` can assign it. That keeps
every function pure from the outside: a variable's changes never escape the
function that owns it.

    tally = |items| {
      let mut total = 0
      let add = |n| total = total + n    // error
      for i in items { total = total + i.amount }   // fix: assign it here
      total
    }",
    },
    Entry {
        code: "assign",
        title: "Assigning something that isn't a `state` or `let mut`",
        text: "In a handler or lambda, `name = value` changes a component's `state` or a
`let mut` variable. This name is neither.",
    },
    Entry {
        code: "refutable",
        title: "A pattern that can fail, without `else`",
        text: "`let Some(list) = lists.get(id)` doesn't say what happens when the result is
`None`. Handle the other cases with `let … else`, or with `match`.

    let Some(list) = lists.get(id)                                   // error
    let Some(list) = lists.get(id) else { return Err(NoSuchList) }   // fix",
    },
    Entry {
        code: "let-else",
        title: "The `else` of `let … else` doesn't leave",
        text: "When the pattern doesn't match, the names it would bind don't exist, so the
code after the `let` can't run. The `else` block must leave: end it with
`return`, `break`, `continue` or `crash`.

    let Some(item) = items.get(id) else { {} }                        // error
    let Some(item) = items.get(id) else { return Err(NoSuchItem) }   // fix",
    },
    Entry {
        code: "or-pattern",
        title: "The sides of `|` bind different names",
        text: "An arm with `A | B` runs for either side, so the names it binds must exist
whichever side matched.

    Ok(x) | Err(y) -> …        // error: x or y might not exist
    Ok(v) | Err(v) -> …        // fix, when both carry the same type",
    },
    Entry {
        code: "state-outside",
        title: "`state` outside a component",
        text: "`state` belongs to a component, or to a stateful function that a component
calls. Elsewhere, use `let` or `let mut`.",
    },
    Entry {
        code: "task-placement",
        title: "A `task` inside other code",
        text: "A `task` starts when its component appears and stops when it goes away, so it
belongs at the top of a component or stateful function, not inside an `if`, a
loop or a handler.",
    },
    Entry {
        code: "stateful-call",
        title: "A stateful function called conditionally",
        text: "A function that declares `state` or `task` keeps its state between renders by
being called in the same place every time. Call it at the top of a component
or another stateful function, never inside an `if`, a loop or a lambda.",
    },
    Entry {
        code: "unused-result",
        title: "A Result is ignored",
        text: "This call can fail, and nothing handles the failure. Wait for it and handle
the Result.

    save = || rename_item(item.id, draft)                                   // error
    save = || rename_item(item.id, draft).await.on_err(|err| problem = err) // fix",
    },
    Entry {
        code: "untyped-remote",
        title: "A query or command without types",
        text: "`Query(…)` and `Command(…)` are the API between client and server, so their
parameter types and result type are written, not inferred.

    items_in = Query(|list, show| …)                                      // error
    items_in = Query(|list: Id(TodoList), show: Filter| -> List(Item) { … }) // fix",
    },
    Entry {
        code: "table-method",
        title: "A method tables don't have",
        text: "See the table API in the reference (Data, Reading and Writing). Every List
method works on a table's rows, and `rows()` gives them as a list.",
    },
    Entry {
        code: "query-fragment",
        title: "A function that can't run as a database query",
        text: "Functions given to table methods compile to SQL, so they can only use the
query fragment: field access, comparisons, arithmetic, `and`/`or`/`not`,
`if`/`match`, query methods, other tables, and helpers built from those. No
`let mut`, loops or recursion. To use anything else, fetch the rows first and
work in memory, where it's visible:

    items.filter(|i| fancy_score(i) > 3)            // error: fancy_score loops
    items.rows().filter(|i| fancy_score(i) > 3)     // fix: in memory, on purpose",
    },
    Entry {
        code: "expect-effect",
        title: "`expect` with an effect",
        text: "`expect` states a fact about pure code, so it can't read tables, the clock or
randomness.",
    },
    Entry {
        code: "constant-effect",
        title: "A top-level constant with an effect",
        text: "Top-level constants are computed once, not each time something runs, so
they can't read tables, the clock or randomness. Make it a function.

    total = items.count()          // error
    total = || items.count()       // fix",
    },
    Entry {
        code: "query-effect",
        title: "A query that does more than read",
        text: "A `Query` only reads tables. To change data, write a `Command`.",
    },
    Entry {
        code: "check-effect",
        title: "A check that does more than read",
        text: "A `Check` states something that must always be true of the data, so it only
reads.",
    },
    Entry {
        code: "command-effect",
        title: "A command that waits or reaches outside",
        text: "A command is atomic and may be retried, so it can't wait or call other
services: neither can be undone or safely repeated. Fetch first, in a
handler, task or job, and pass the result in; or record what needs doing in a
table and let a job do it (the outbox pattern, in Externs, jobs and
endpoints).",
    },
    Entry {
        code: "view-effect",
        title: "A view that changes something",
        text: "A component's `view` and its bindings only read: they call queries and read
`Device` or `Memory` tables. Changes happen in handlers.

    view = p { send_it() }                       // error
    view = button(on_click: send_it) { \"Send\" }  // fix: a handler",
    },
    Entry {
        code: "server-only",
        title: "Client code using a server table",
        text: "Shared tables live in the server's database. Client code reaches them through
queries and commands, which also keeps them out of the browser bundle.

    view = p { \"${items.count()} items\" }         // error
    open_count = Query(|| -> Int { items.count() })
    view = p { \"${open_count()} items\" }          // fix",
    },
    Entry {
        code: "type",
        title: "Types don't fit",
        text: "A value of one type is used where another is needed. The message says which
two types disagree. Tag unions grow by inference, so a tag that isn't allowed
usually means a closed union (a parameter's type, or a `match` with no
catch-all arm) doesn't list it.",
    },
];
