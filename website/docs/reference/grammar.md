---
title: Keywords, operators and grammar
description: Every keyword and operator, and the grammar of the language.
---

# Keywords, operators and grammar

## Keywords

| Keyword | Meaning | See |
|---|---|---|
| `and`, `or`, `not` | logical operators | [Expressions](expressions#operators) |
| `await` | after a call, with `.`: waits for its value | [UI: waiting](ui#waiting-await) |
| `break`, `continue` | leave or restart a loop | [Control flow](control-flow#loops) |
| `component` | declares a UI component | [UI](ui#components) |
| `crash` | stops with a runtime error | [Runtime errors](runtime-errors#crash) |
| `defer` | runs a block when the enclosing block ends | [Control flow](control-flow#defer) |
| `effect` | declares a user-defined effect | [Effects](effects#user-defined-effects) |
| `else` | the other branch of `if` | [Control flow](control-flow#if-and-else) |
| `expect` | states a fact about pure code | [Checks and tests](checks-and-tests#expect) |
| `for`, `in` | loops | [Control flow](control-flow#loops), [UI](ui#loops) |
| `handler` | implements an effect | [Effects](effects#user-defined-effects) |
| `if` | conditional | [Control flow](control-flow#if-and-else) |
| `match` | pattern matching | [Control flow](control-flow#match) |
| `record` | declares a nominal record type | [Types](types#records) |
| `return` | leaves a function early | [Control flow](control-flow#early-return) |
| `state` | declares component state | [UI](ui#state) |
| `style` | a component's scoped CSS | [UI](ui#styling) |
| `let` | binds a local name; `let mut` declares a variable | [Bindings](bindings) |
| `where` | constrains type variables | [Functions](functions#where-clauses) |
| `while` | loops while a condition holds | [Control flow](control-flow#loops) |
| `with` | installs an effect handler | [Effects](effects#user-defined-effects) |

Contextual keywords: `key` (in a `record`), `view` (in a `component`),
`task` (in a `component` or stateful function), `ctl` and `final` (in an
`effect`).

## Operators and symbols

| Symbol | Meaning |
|---|---|
| `=` | definition; after `let`, a binding; in a block, assignment to a `let mut` or `state` |
| `:` | type annotation; record field |
| `->` | function type; `match` arm |
| `\|a, b\|` | lambda parameters |
| `.` | field access; method call; `.field` as a function |
| `..` | in a record: copy the rest (`{ ..r, x: 1 }`); in a list pattern: the rest |
| `..<`, `..=` | ranges |
| `?` | unwrap a `Result` or return its error |
| `+ - * / %` | arithmetic |
| `== != < <= > >=` | comparison |
| `//`, `/* */` | comment; block comment (nests) |
| `///`, `//!` | doc comment for the next declaration; for the file |
| `${ }` | text interpolation |

## Grammar

The grammar in EBNF. `{ x }` means zero or more, `[ x ]` means optional, and
`NL` is a line break. Line continuation follows
[Lexical structure: lines](lexical#lines-and-blocks).

```text
file          = { NL } { top_level { NL } }

top_level     = record | component | effect | handler | signature
              | binding | alias | expect

record        = "record" Upper "{" { NL } { record_item NL } "}"
record_item   = [ "key" ] lower ":" type [ "=" expr ]
              | lower "=" expr                                   (* a method *)

component     = "component" Upper [ "(" params ")" ] "{" { NL } { component_item NL } "}"
component_item = state_decl | task
              | "view" "=" markup
              | "style" "{" css "}"
              | binding
state_decl    = "state" lower [ ":" type ] "=" expr
task          = "task" [ args ] block

effect        = "effect" Upper "{" { NL } { [ "final" | "ctl" ] lower ":" type NL } "}"
handler       = "handler" lower ":" Upper "{" { NL } { binding NL } "}"

signature     = name ":" type [ "where" "[" constraints "]" ]
binding       = name "=" expr
alias         = Upper [ "(" lower { "," lower } ")" ] ":" type
expect        = "expect" expr

type          = fn_type | value_type
fn_type       = value_type { "," value_type } "->" value_type
value_type    = Upper [ "(" type { "," type } ")" ]
              | lower
              | "{" [ field_type { "," field_type } ] "}"
              | "[" tag_type { "," tag_type } "]"
              | "(" ")"

block         = "{" { NL } { statement NL } "}"
statement     = let | assignment
let           = "let" [ "mut" ] pattern "=" expr [ "else" block ]
              | "if" ( expr | "let" pattern "=" expr ) block [ "else" ( block | if_stmt ) ]
              | "for" pattern "in" expr block
              | "while" expr block
              | "break" | "continue" | "return" [ expr ]
              | "with" expr block
              | "defer" block
              | state_decl | task                     (* in a stateful function *)
              | expr
assignment    = lower "=" expr

expr          = or_expr
or_expr       = and_expr { "or" and_expr }
and_expr      = cmp_expr { "and" cmp_expr }
cmp_expr      = range_expr [ cmp_op range_expr ]
range_expr    = add_expr [ ( "..<" | "..=" ) add_expr ]
add_expr      = mul_expr { ( "+" | "-" ) mul_expr }
mul_expr      = unary { ( "*" | "/" | "%" ) unary }
unary         = [ "-" | "not" ] postfix
postfix       = primary { "." lower [ args ] | "." "await" | args | "?" }
primary       = literal | name | "." lower | lambda | block | record_lit
              | list_lit | tag | if_expr | match_expr | "crash" expr | "(" expr ")"

lambda        = "|" [ param { "," param } ] "|" [ "->" type ] expr
param         = pattern [ ":" type ]
if_expr       = "if" ( expr | "let" pattern "=" expr ) block "else" ( block | if_expr )
match_expr    = "match" expr "{" { NL } { pattern { "|" pattern } [ "if" expr ] "->" expr NL } "}"
record_lit    = [ Upper ] "{" [ field { "," field } [ "," ] ] "}"
field         = lower [ ":" expr ] | ".." expr
args          = "(" [ arg { "," arg } [ "," ] ] ")"
arg           = [ lower ":" ] expr

markup        = element | Upper [ args ] | text | "nothing" | "outlet"
              | "if" ( expr | "let" pattern "=" expr ) markup_block [ "else" markup_block ]
              | "match" expr "{" { pattern "->" markup NL } "}"
              | "for" pattern "in" expr markup_block
element       = lower [ args ] [ markup_block ]
markup_block  = "{" { NL } { markup NL } "}"

name          = lower | Upper
```

The executable form of this grammar is `tree-sitter-weft/grammar.js`, which
also records how each ambiguity is resolved (see its README). The grammar
describes structure only. Which names are in scope, which
folders allow which declarations, and every rule in the other pages are
checked after parsing.
