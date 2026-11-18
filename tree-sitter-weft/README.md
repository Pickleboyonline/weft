# tree-sitter-weft

The grammar for weft, written for [tree-sitter](https://tree-sitter.github.io/).
It's the executable form of the language reference (`website/docs/reference/`):
`make grammar` regenerates `src/parser.c` from `grammar.js`, and tree-sitter
refuses to generate a parser when two rules can match the same input. The
project declares no conflicts, so every ambiguity in the syntax has to be
resolved here, with a rule that says which reading wins.

`src/scanner.c` handles what a grammar can't: a line break ends a statement
unless the line obviously continues (it ends with an operator, comma or open
bracket, or the next line starts with `.` or `else`), the CSS inside
`style { … }`, and nested `/* */` comments.

## Resolutions

Each is marked `RESOLUTION n` in `grammar.js`. They are choices the reference
didn't make; the project owner may want to make them differently.

| # | The ambiguity | How it's resolved |
|---|---|---|
| 1 | `name { … }` after `if`, `while`, `for … in`, `match` or `with`: an element (or typed record `Name { … }`) with children, or the expression followed by its block? | The block. An element or typed record can't be the condition, iterable, scrutinee or handler without parentheses, as in Rust. |
| 2 | Markup children: a separate syntax? | No: an element's children are a block of statements, so `if`, `for` and `match` in markup are the ordinary ones. |
| 3 | `|a: Int, b: Int|`: does the comma end the type, or continue a function type `Int, b -> …`? | Where a comma separates something else (parameters, fields, type arguments), only a value type is allowed. A function type there needs a name, e.g. `Action(a)`. |
| 4 | `{ x }`: a block whose value is `x`, or a record with the shorthand field `x`? | A block. A record needs a `:` field, a `..spread`, or two or more fields. A one-field shorthand record is `{ x: x }`, or `{ x, }`. |
| 5 | `{}`: an empty block or the empty record? | The empty record, as the reference says. An empty block only appears where the grammar requires a block (`if x {}`). |
| 6 | *(Gone.)* `|| x = 1` in a block was a lambda or a binding whose left side is `|| x`. New bindings now start with `let`, so a binding's left side is always a name, and the ambiguity no longer exists. | |
| 7 | `|x| -> Name (y)`: does `(y)` continue the result type, or start the body? | The type: `Name(y)`. Lambdas with a result type normally have a block body. |
| 9 | `let x = if c { a } else { b }`: does the `else` belong to the `if`, or make this a `let … else`? | To the `if`, as in Rust. A `let … else` whose value ends in `if` or `match` puts the value in parentheses. |
| 8 | `where [a.compare : a, a -> X, …]`: does the comma after the first `a` end the constraint? | No, it continues the function type's parameters. A non-function constraint can only come last in a `where` list. |

One place where the grammar is looser than the reference, with the checker
enforcing the rule instead: chained comparisons (`a < b < c`), since
tree-sitter has no non-associative operators.
