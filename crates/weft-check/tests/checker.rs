//! The checker against the example project (no errors) and against planted
//! bugs (each caught, with the right code).

use std::path::Path;

use weft_check::{check, load_project, source};

fn example() -> Vec<weft_check::Source> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/todo-lists");
    load_project(&dir).expect("the example project is readable")
}

/// Checks the example plus one extra file, and returns `(line, code)` for
/// every error in that file (1-based lines).
fn errors_in(extra: &str) -> Vec<(usize, &'static str)> {
    let mut sources = example();
    sources.push(source("extra.weft", extra));
    let last = sources.len() - 1;
    check(&sources, true)
        .diagnostics
        .into_iter()
        .filter(|(f, _)| *f == last)
        .map(|(_, d)| (d.span.line + 1, d.code))
        .collect()
}

fn assert_has(errors: &[(usize, &str)], line: usize, code: &str) {
    assert!(errors.contains(&(line, code)), "expected {code} on line {line}, got {errors:?}");
}

#[test]
fn the_example_has_no_errors() {
    let sources = example();
    let report = check(&sources, true);
    let msgs: Vec<String> = report.diagnostics.iter().map(|(f, d)| d.render(&sources[*f].path, &sources[*f].text)).collect();
    assert!(msgs.is_empty(), "{}", msgs.join("\n"));
}

#[test]
fn types() {
    let e = errors_in(
        r#"component Bugs(list: TodoList) {
  state count = 0
  bad_args = items_in(list.id, 3)
  bad_field = list.title
  wrong_component = ItemRow(itm: list)
  sum = count + "x"
  view = p { "${bad_args}" }
}
"#,
    );
    assert_has(&e, 3, "type");
    assert_has(&e, 4, "type");
    assert_has(&e, 5, "type");
    assert_has(&e, 6, "type");
}

#[test]
fn effects_where_they_are_not_allowed() {
    let e = errors_in(
        r#"total = items.count()

peek = Query(|| -> Int {
  now()
  items.count()
})

ping = Command(|| -> {} {
  http.get("https://example.com").await
  {}
})

component Leaky {
  view = p { "${items.count()} items" }
}
"#,
    );
    assert_has(&e, 1, "constant-effect");
    assert_has(&e, 4, "query-effect");
    assert_has(&e, 9, "command-effect");
    assert_has(&e, 14, "server-only");
}

#[test]
fn results_cant_be_ignored() {
    let e = errors_in(
        r#"component Rename(item: Item) {
  state draft = ""
  save = || rename_item(item.id, draft)
  view = p { draft }
}
"#,
    );
    assert_has(&e, 3, "unused-result");
}

#[test]
fn queries_must_fit_the_fragment() {
    let e = errors_in(
        r#"score = |i| {
  var n = 0
  while n < 3 { n = n + 1 }
  n
}

high = Query(|| -> List(Item) { items.filter(|i| score(i) > 1) })
"#,
    );
    assert_has(&e, 7, "query-fragment");
}

#[test]
fn remote_functions_are_typed() {
    let e = errors_in("loose = Query(|id| lists.get(id))\n");
    assert_has(&e, 1, "untyped-remote");
}

#[test]
fn scoping() {
    let e = errors_in(
        r#"f = |items| {
  let cleaned = "a"
  cleaned = "b"
  let cleaned = cleaned.trim()
  let mut total = 0
  let add = |n| total = total + n
  undeclared = 1
  total
}
"#,
    );
    assert_has(&e, 3, "immutable");
    assert_has(&e, 6, "var-escape");
    assert_has(&e, 7, "unknown-name");
    // Shadowing with `let`, and a parameter named like the `items` table, are fine.
    assert!(!e.iter().any(|(l, _)| *l == 1 || *l == 4), "{e:?}");
}

#[test]
fn match_must_cover_every_case() {
    let e = errors_in(
        r#"label = |f: Filter| match f {
  All    -> "all"
  Active -> "active"
}
"#,
    );
    assert_has(&e, 1, "type");
}

#[test]
fn let_else_if_let_and_or_patterns() {
    let e = errors_in(
        r#"move_item = Command(|id: Id(Item), to: Id(TodoList)| -> Result({}, [NoSuchItem, EmptyTitle, NoSuchList]) {
  let Some(item) = items.get(id) else { return Err(NoSuchItem) }
  add_item(to, item.title)?
  Ok({})
})
stays = Command(|id: Id(Item)| -> {} {
  let Some(item) = items.get(id) else { {} }
  items.delete(item.id)
})
label = |show: Filter, problem: [None, Oops(Text)]| {
  if let Oops(why) = problem { why } else { "" }
  match show {
    All           -> "all"
    Active | Done -> "some"
  }
}
mixed = |r| match r {
  Ok(x) | Err(y) -> 1
  _ -> 2
}
"#,
    );
    assert_has(&e, 7, "let-else");
    assert_has(&e, 18, "or-pattern");
    assert!(!e.iter().any(|(l, _)| *l <= 5 || (11..=15).contains(l)), "{e:?}");
}

#[test]
fn every_code_has_an_explanation() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/walk.rs")).unwrap()
        + &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/rules.rs")).unwrap()
        + &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/program.rs")).unwrap()
        + &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/types.rs")).unwrap();
    for e in weft_check::explain::ENTRIES {
        assert!(!e.text.is_empty(), "{}", e.code);
    }
    for code in ["immutable", "var-escape", "refutable", "let-else", "or-pattern", "unused-result", "query-fragment",
        "server-only", "view-effect", "command-effect", "query-effect", "check-effect", "constant-effect",
        "expect-effect", "untyped-remote", "stateful-call", "task-placement", "state-outside", "table-method",
        "unknown-name", "duplicate", "assign", "type"]
    {
        assert!(src.contains(&format!("\"{code}\"")), "{code} isn't emitted");
        assert!(weft_check::explain::lookup(code).is_some(), "{code} has no explanation");
    }
}
