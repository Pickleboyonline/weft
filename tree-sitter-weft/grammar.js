/**
 * tree-sitter grammar for weft.
 *
 * The language reference (website/docs/reference/) is the specification. This
 * grammar is its executable form: `tree-sitter generate` fails on any
 * ambiguity, and the project allows no declared conflicts, so every ambiguity
 * has to be resolved here with an explicit, documented rule. Each such rule is
 * marked RESOLUTION below and listed in README.md.
 */

const PREC = {
  or: 1,
  and: 2,
  compare: 3,
  range: 4,
  add: 5,
  mul: 6,
  unary: 7,
  postfix: 8,
};

const commaSep1 = (rule) => seq(rule, repeat(seq(',', rule)));
const commaSep = (rule) => optional(commaSep1(rule));
// Newlines inside braces that may also start a block: emitted by the scanner,
// skipped here (see RESOLUTION 4).
const nl = ($) => optional($._nls);
// Fields after the first: `, field` repeated, then an optional trailing comma,
// with line breaks allowed after a comma or before the `}`.
const moreFields = ($) => seq(
  repeat(seq(',', nl($), $._field)),
  optional(choice(seq(',', nl($)), $._nls)),
);

module.exports = grammar({
  name: 'weft',

  externals: ($) => [$._newline, $.css_body, $.block_comment],

  extras: ($) => [/[ \t\r\n]/, $.line_comment, $.block_comment],

  word: ($) => $.identifier,

  supertypes: ($) => [$._expression, $._pattern, $._type, $._statement],

  rules: {
    source_file: ($) => seq(nl($), optional(seq(
      $._top_level,
      repeat(seq($._nls, $._top_level)),
      nl($),
    ))),

    _nls: ($) => repeat1($._newline),

    _top_level: ($) => choice(
      $.record_declaration,
      $.component_declaration,
      $.effect_declaration,
      $.handler_declaration,
      $.signature,
      $.alias,
      $.binding,
      $.expect,
    ),

    // ── Comments ──────────────────────────────────────────────

    line_comment: (_) => token(seq('//', /[^\n]*/)),

    // ── Declarations ──────────────────────────────────────────

    record_declaration: ($) => seq(
      'record', field('name', $.type_identifier),
      '{', nl($), optional($._record_items), '}',
    ),
    _record_items: ($) => seq(
      $._record_item, repeat(seq($._nls, $._record_item)), nl($),
    ),
    _record_item: ($) => choice($.record_field, $.binding),
    record_field: ($) => seq(
      optional(field('key', 'key')),
      field('name', $.identifier), ':', field('type', $._value_type),
      optional(seq('=', field('default', $._expression))),
    ),

    component_declaration: ($) => seq(
      'component', field('name', $.type_identifier),
      optional(field('parameters', $.parameter_list)),
      field('body', $.block),
    ),
    parameter_list: ($) => seq('(', commaSep($.component_parameter), optional(','), ')'),
    component_parameter: ($) => seq(
      field('name', $.identifier), ':', field('type', $._value_type),
      optional(seq('=', field('default', $._expression))),
    ),

    effect_declaration: ($) => seq(
      'effect', field('name', $.type_identifier),
      '{', nl($), optional(seq(
        $.operation, repeat(seq($._nls, $.operation)), nl($),
      )), '}',
    ),
    operation: ($) => seq(
      optional(field('kind', choice('final', 'ctl'))),
      field('name', $.identifier), ':', field('type', $._type),
    ),

    handler_declaration: ($) => seq(
      'handler', field('name', $.identifier), ':', field('effect', $.type_identifier),
      field('body', $.block),
    ),

    signature: ($) => seq(
      field('name', $.identifier), ':', field('type', $._type),
      optional(field('where', $.where_clause)),
    ),
    where_clause: ($) => seq('where', '[', commaSep1($.constraint), ']'),
    constraint: ($) => seq(
      field('variable', $.identifier), '.', field('method', $.identifier),
      ':', field('type', $._type),
    ),

    alias: ($) => seq(
      field('name', $.type_identifier),
      optional(seq('(', commaSep1($.identifier), ')')),
      ':', field('type', $._type),
    ),

    // `name = value`: a top-level definition, a record method, a component
    // member, or, inside a block, an assignment to a `let mut` or a `state`.
    // New local bindings use `let`.
    binding: ($) => seq(field('left', $.identifier), '=', field('value', $._expression)),

    // `let pattern = value` binds; `let mut name = value` makes a variable.
    // A `let` may shadow any name already in scope.
    // `let pattern = value else { … }` handles the case the pattern doesn't
    // match; the `else` block must leave (return, break, continue or crash).
    let_declaration: ($) => seq(
      'let', optional(field('mutable', 'mut')),
      field('pattern', $._pattern), '=', field('value', $._expression),
      optional(seq('else', field('alternative', $.block))),
    ),

    expect: ($) => seq('expect', field('condition', $._expression)),

    // ── Statements and blocks ─────────────────────────────────

    // A block holds one statement per line. Markup children are statements
    // too, so an element's children are a block (RESOLUTION 2).
    block: ($) => seq('{', nl($), optional($._statements), '}'),
    // RESOLUTION 5. In expression position `{}` is the empty record (the
    // reference: "`{}` is the empty record"), so a block used as a value has
    // at least one statement. An empty block is still allowed where the
    // grammar requires a block, such as `if x {}`.
    block_expression: ($) => seq('{', nl($), $._statements, '}'),
    _statements: ($) => seq(
      $._statement, repeat(seq($._nls, $._statement)), nl($),
    ),

    _statement: ($) => choice(
      $.binding,
      $.let_declaration,
      $.state_declaration,
      $.style_block,
      $.for_statement,
      $.while_statement,
      $.break_statement,
      $.continue_statement,
      $.return_statement,
      $.with_statement,
      $.defer_statement,
      $._expression,
    ),

    state_declaration: ($) => seq(
      'state', field('name', $.identifier),
      optional(seq(':', field('type', $._value_type))),
      '=', field('value', $._expression),
    ),
    style_block: ($) => seq('style', '{', optional($.css_body), '}'),
    for_statement: ($) => seq(
      'for', field('pattern', $._pattern), 'in', field('iterable', $._expression),
      field('body', $.block),
    ),
    while_statement: ($) => seq('while', field('condition', $._expression), field('body', $.block)),
    break_statement: (_) => 'break',
    continue_statement: (_) => 'continue',
    return_statement: ($) => prec.right(seq('return', optional($._expression))),
    with_statement: ($) => seq('with', field('handler', $._expression), field('body', $.block)),
    defer_statement: ($) => seq('defer', field('body', $.block)),

    // ── Expressions ───────────────────────────────────────────

    _expression: ($) => choice(
      $.identifier,
      $.type_identifier,
      $.number,
      $.text,
      $.list,
      $.record,
      $.typed_record,
      alias($.block_expression, $.block),
      $.lambda,
      $.if_expression,
      $.match_expression,
      $.accessor,
      $.parenthesized,
      $.unit,
      $.call,
      $.element,
      $.field_access,
      $.await_expression,
      $.try_expression,
      $.unary_expression,
      $.binary_expression,
      $.range_expression,
      $.crash_expression,
    ),

    // `crash` has every type, so it fits in any branch (reference: Runtime
    // errors). The EBNF listed it as a statement; it's an expression.
    crash_expression: ($) => prec.right(seq('crash', $._expression)),

    parenthesized: ($) => seq('(', $._expression, ')'),
    unit: (_) => seq('(', ')'),

    // `.created`, a function from a record to its field.
    accessor: ($) => seq('.', field('field', $.identifier)),

    call: ($) => prec(PREC.postfix, seq(field('function', $._expression), field('arguments', $.arguments))),
    arguments: ($) => seq('(', commaSep($.argument), optional(','), ')'),
    argument: ($) => choice(
      seq(field('name', $.identifier), ':', field('value', $._argument_value)),
      field('value', $._argument_value),
    ),
    // A lambda passed as an argument may assign state: `on_click: || open = True`.
    _argument_value: ($) => choice($._expression, $.assignment),

    // RESOLUTION 1. A name, call or tag followed by `{ … }` is an element with
    // children, except where the expression is followed by a block anyway:
    // the condition of `if` and `while`, the iterable of `for`, the scrutinee
    // of `match`, and the handler of `with`. There the `{` starts that block,
    // as in Rust. The negative precedence makes the parser end the expression
    // instead of starting an element.
    element: ($) => prec(-1, seq(
      field('name', choice($.identifier, $.call)),
      field('children', $.block),
    )),

    field_access: ($) => prec(PREC.postfix, seq(
      field('value', $._expression), '.', field('field', $.identifier),
    )),
    await_expression: ($) => prec(PREC.postfix, seq(field('value', $._expression), '.', 'await')),
    try_expression: ($) => prec(PREC.postfix, seq(field('value', $._expression), '?')),

    unary_expression: ($) => prec(PREC.unary, seq(
      field('operator', choice('-', 'not')), field('operand', $._expression),
    )),

    binary_expression: ($) => choice(
      ...[
        ['or', PREC.or],
        ['and', PREC.and],
        ['+', PREC.add], ['-', PREC.add],
        ['*', PREC.mul], ['/', PREC.mul], ['%', PREC.mul],
      ].map(([op, p]) => prec.left(p, seq(
        field('left', $._expression), field('operator', op), field('right', $._expression),
      ))),
      // Comparisons don't chain. The parser accepts `a < b < c` and the
      // checker reports it, since tree-sitter has no non-associative operators.
      ...['==', '!=', '<', '<=', '>', '>='].map((op) => prec.left(PREC.compare, seq(
        field('left', $._expression), field('operator', op), field('right', $._expression),
      ))),
    ),

    range_expression: ($) => prec.left(PREC.range, seq(
      field('start', $._expression), field('operator', choice('..<', '..=')), field('end', $._expression),
    )),

    // RESOLUTION 4. `{` starts a block or a record. A record is chosen when
    // the braces are empty, or hold a field with `:`, a `..spread`, or at
    // least two fields; `{ x }` is a block whose value is `x`. Newlines
    // between a record's fields are allowed, because at `{` the scanner can't
    // yet tell which one it is.
    record: ($) => seq('{', nl($), optional(choice(
      seq(choice($.field, $.spread), moreFields($)),
      seq($.shorthand_field, ',', nl($), optional(seq($._field, moreFields($)))),
    )), '}'),
    // RESOLUTION 1 covers typed records too: `while True { … }` loops, it
    // doesn't build a record of type `True`.
    typed_record: ($) => prec(-1, seq(
      field('type', $.type_identifier),
      '{', nl($), optional(seq($._field, moreFields($))), '}',
    )),
    _field: ($) => choice($.field, $.shorthand_field, $.spread),
    field: ($) => seq(field('name', $.identifier), ':', field('value', $._expression)),
    shorthand_field: ($) => field('name', $.identifier),
    spread: ($) => seq('..', field('value', $._expression)),

    list: ($) => seq('[', commaSep($._expression), optional(','), ']'),

    lambda: ($) => prec.right(seq(
      choice('||', seq('|', commaSep1($.parameter), '|')),
      optional(seq('->', field('result', $._value_type))),
      field('body', choice($._expression, $.assignment)),
    )),
    parameter: ($) => seq(field('pattern', $._pattern), optional(seq(':', field('type', $._value_type)))),

    // An assignment used where an expression is expected: a lambda's body, an
    // argument, or a `match` arm. `|| editing = False`.
    assignment: ($) => prec.right(seq(field('left', $.identifier), '=', field('value', $._expression))),

    // RESOLUTION 9. In `let x = if c { a } else { b }`, the `else` belongs to
    // the `if`, not to a `let … else`. A `let … else` whose value is an `if`
    // or `match` needs parentheses around the value, as in Rust.
    if_expression: ($) => prec.right(1, seq(
      'if', field('condition', choice($._expression, $.let_condition)), field('consequence', $.block),
      optional(seq('else', field('alternative', choice($.block, $.if_expression)))),
    )),

    match_expression: ($) => seq(
      'match', field('value', $._expression), '{', nl($),
      optional(seq($.match_arm, repeat(seq($._nls, $.match_arm)), nl($))),
      '}',
    ),
    // `if let Some(x) = value { … }`.
    let_condition: ($) => seq('let', field('pattern', $._pattern), '=', field('value', $._expression)),

    match_arm: ($) => seq(
      field('pattern', choice($._pattern, $.or_pattern)),
      optional(seq('if', field('guard', $._expression))),
      '->', field('value', choice($._expression, $.assignment)),
    ),

    // ── Patterns ──────────────────────────────────────────────

    _pattern: ($) => choice(
      $.identifier,
      $.type_identifier,
      $.tag_pattern,
      $.record_pattern,
      $.list_pattern,
      $.number,
      $.text,
    ),
    // `Active | Done`. Only in `match` arms: in a lambda's parameters `|`
    // already closes the list.
    or_pattern: ($) => seq($._pattern, repeat1(seq('|', $._pattern))),

    tag_pattern: ($) => seq(field('tag', $.type_identifier), '(', commaSep1($._pattern), ')'),
    record_pattern: ($) => seq('{', commaSep1(choice(
      $.identifier,
      seq(field('name', $.identifier), ':', field('pattern', $._pattern)),
    )), optional(','), '}'),
    list_pattern: ($) => seq('[', commaSep(choice($._pattern, $.rest_pattern)), ']'),
    rest_pattern: ($) => seq('..', optional($.identifier)),

    // ── Types ─────────────────────────────────────────────────

    _type: ($) => choice($.function_type, $._value_type),
    // RESOLUTION 8. In `where [a.compare : a, a -> Ordering, …]` the comma
    // after the first `a` could end the constraint or continue the function
    // type's parameters. It continues the parameters. So in a `where` list, a
    // constraint whose type isn't a function can only come last.
    function_type: ($) => prec.right(1, seq(
      field('parameters', commaSep1($._value_type)), '->', field('result', $._value_type),
    )),
    // RESOLUTION 3. Where a type is followed by a comma that separates
    // something else (lambda and component parameters, record fields, type
    // arguments), only a value type is allowed: a function type there must be
    // named, e.g. `Action(a)`.
    _value_type: ($) => choice(
      $.type_application,
      $.type_identifier,
      $.type_variable,
      $.record_type,
      $.tag_union_type,
      $.unit_type,
    ),
    // RESOLUTION 7. In `|x| -> Name (y)`, `(y)` continues the result type
    // (`Name(y)`) rather than starting the body. A lambda with a result type
    // usually has a block body, which has no such problem.
    type_application: ($) => prec(1, seq(
      field('name', $.type_identifier), '(', commaSep1($._type_argument), ')',
    )),
    _type_argument: ($) => $._value_type,
    type_variable: ($) => alias($.identifier, $.type_variable_name),
    record_type: ($) => seq('{', commaSep(seq(field('name', $.identifier), ':', field('type', $._value_type))), optional(','), '}'),
    tag_union_type: ($) => seq('[', commaSep1(choice($.type_identifier, $.tag_type, '..')), ']'),
    tag_type: ($) => seq($.type_identifier, '(', commaSep1(choice($._value_type, '_')), ')'),
    unit_type: (_) => seq('(', ')'),

    // ── Literals and names ────────────────────────────────────

    identifier: (_) => /[a-z_][A-Za-z0-9_]*/,
    type_identifier: (_) => /[A-Z][A-Za-z0-9_]*/,

    // A suffix must touch the number: `10%` is a percentage, `10 % 3` a
    // remainder.
    number: (_) => token(seq(
      /[0-9][0-9_]*/,
      optional(/\.[0-9][0-9_]*/),
      optional(choice('ns', 'us', 'ms', 's', 'min', 'h', 'd', 'w', 'B', 'KB', 'MB', 'GB', 'TB', 'f', '%')),
    )),

    text: ($) => choice(
      seq('"', repeat(choice($.text_content, $.escape, $.interpolation)), token.immediate('"')),
      seq('"""', repeat(choice($.long_text_content, $.long_text_quote, $.escape, $.interpolation)), '"""'),
    ),
    // No precedence here: at `${` the longer interpolation token must win.
    text_content: (_) => token.immediate(/[^"\\$\n]+|\$/),
    long_text_content: (_) => token.immediate(prec(1, /([^"\\$]|\$[^{])+/)),
    // One or two quotes inside a long text; three end it (longest match wins).
    long_text_quote: (_) => token.immediate(/""?/),
    escape: (_) => token.immediate(/\\(u\{[0-9A-Fa-f]+\}|[nrt"\\$])/),
    interpolation: ($) => seq(token.immediate('${'), $._expression, '}'),
  },
});
