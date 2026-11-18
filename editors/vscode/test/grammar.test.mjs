// Grammar tests. Run with: make vscode-test
//
// 1. Scope checks: specific tokens in real weft snippets get the scope we expect.
// 2. Coverage: every non-space character in every .weft file and every ```weft
//    block in the docs and the site is claimed by some rule.
import { highlighter, rawTokens, weftFiles, docBlocks, readFileSync, relative, repoRoot } from "./shared.mjs";

const hl = await highlighter();
let failures = 0;

function fail(msg) {
  failures++;
  console.log(`  FAIL ${msg}`);
}

// expect(title, code, [text, scope, nth?]...): the nth token whose text is
// `text` must have a scope starting with `scope`. A leading "!" means it must NOT.
function expect(title, code, checks, lang = "weft") {
  const toks = rawTokens(hl, code, lang);
  for (const [text, scope, nth = 0] of checks) {
    const tok = toks.filter((t) => t.text.trim() === text)[nth];
    if (!tok) {
      fail(`${title}: no token "${text}" (#${nth})`);
      continue;
    }
    const negate = scope.startsWith("!");
    const want = negate ? scope.slice(1) : scope;
    const has = tok.scopes.some((s) => s === want || s.startsWith(want + "."));
    if (has === negate) {
      fail(`${title}: "${text}" ${negate ? "should not have" : "missing"} ${want}\n         got ${tok.scopes.join(" ")}`);
    }
  }
}

console.log("scope checks");

expect("record with key, defaults and a method",
  `/// One item on a list.
record Item {
  key id:  Id(Item) = new_id()
  title:   Text
  done:    Bool = False

  label = |i| if i.done { "done" } else { i.title }
}`,
  [
    ["One item on a list.", "comment.line.documentation"],
    ["record", "storage.type.record"],
    ["Item", "entity.name.type", 0],
    ["key", "storage.modifier.key"],
    ["id", "variable.other.property"],
    ["Id", "support.type.builtin"],
    ["new_id", "entity.name.function"],
    ["False", "constant.language.boolean"],
    ["label", "entity.name.function"],
    ["if", "keyword.control"],
  ]);

expect("tables are values", "items = Table(Item, delete_with: { list: lists })", [
  ["items", "variable.other.definition"],
  ["Table", "support.type.builtin"],
  ["delete_with", "variable.other.property"],
]);

expect("a command: signature and definition",
  `add_item : Id(TodoList), Text -> Result(Id(Item), [EmptyTitle, NoSuchList])
add_item = |list, title| {
  let cleaned = title.trim()
  if cleaned.is_empty() { return Err(EmptyTitle) }
  if not lists.has(list) { return Err(NoSuchList) }
  Ok(items.insert(Item { list, title: cleaned }))
}`,
  [
    ["add_item", "entity.name.function", 0],
    ["->", "keyword.operator.arrow"],
    ["add_item", "entity.name.function", 1],
    ["list", "variable.parameter", 0],
    ["cleaned", "variable.other.definition"],
    ["is_empty", "entity.name.function.member"],
    ["return", "keyword.control", 0],
    ["EmptyTitle", "entity.name.type", 1],
    ["has", "entity.name.function.member"],
    ["insert", "entity.name.function.member"],
  ]);

expect("remote functions are values",
  `add_item = Command(|list: Id(TodoList), title: Text| -> Result(Id(Item), [EmptyTitle]) {
  Ok(items.insert(Item { list, title }))
})
items_in = Query(|list: Id(TodoList)| -> List(Item) { items.filter(|i| i.list == list) })`,
  [
    ["add_item", "variable.other.definition"],
    ["Command", "support.type.builtin"],
    ["Query", "support.type.builtin"],
    ["Result", "support.type.builtin"],
    ["EmptyTitle", "entity.name.type"],
  ]);

expect("a query, with expect",
  `items_in = |list, show| items
  .filter(|i| i.list == list and keeps(show, i))
  .sort_by(.created)

expect keeps(Active, { done: True }) == False`,
  [
    ["items_in", "entity.name.function"],
    ["filter", "entity.name.function.member"],
    ["and", "keyword.operator.wordlike"],
    ["created", "variable.other.property"],
    ["expect", "keyword.other"],
  ]);

expect("a component: state, handlers, view, style",
  `component ItemRow(item: Item) {
  state editing = False

  save = || {
    editing = False
    rename_item(item.id, draft).await.on_err(|err| problem = err)
  }

  task(key: item.id) {
    socket = ws.connect(url).await
    defer { socket.close() }
  }

  view = li {
    checkbox(checked: item.done, on_change: |d| set_done(item.id, d))
    span(class: { done: item.done }, on_click: save) { item.title }
    button(on_click: || editing = False) { "Cancel" }
  }

  style {
    .done { text-decoration: line-through }
  }
}`,
  [
    ["component", "storage.type.component"],
    ["ItemRow", "entity.name.type.class"],
    ["state", "storage.type.state"],
    ["editing", "variable.other.definition", 0],
    ["save", "entity.name.function"],
    ["await", "keyword.control", 0],
    ["on_err", "entity.name.function.member"],
    ["task", "storage.type.task"],
    ["defer", "keyword.control"],
    ["close", "entity.name.function.member"],
    ["view", "storage.type.view"],
    ["li", "entity.name.tag"],
    ["checkbox", "entity.name.tag"],
    ["on_change", "variable.other.property"],
    ["d", "variable.parameter"],
    ["style", "storage.type.style"],
    ["text-decoration", "meta.embedded.block.css"],
    ["text-decoration", "!variable.other.property"],
  ]);

expect("pages, links and the layout",
  `component App {
  view = main {
    Sidebar
    outlet
  }
}
link(to: ListPage(id: row.list.id)) { row.list.name }`,
  [
    ["Sidebar", "entity.name.type"],
    ["outlet", "constant.language"],
    ["link", "entity.name.tag"],
    ["ListPage", "entity.name.type"],
  ]);

expect("loops, variables and early return",
  `first_overdue = |items, today| {
  let mut n = 0
  let { title, done } = first
  for i in items {
    if i.done { continue }
    if i.due < today { return Some(i) }
    while n < 3 { n = n + 1 }
    break
  }
  crash "unreachable"
}`,
  [
    ["let", "storage.type.let", 0],
    ["mut", "storage.modifier.mut"],
    ["n", "variable.other.definition", 0],
    ["for", "keyword.control"],
    ["in", "keyword.control"],
    ["continue", "keyword.control"],
    ["return", "keyword.control"],
    ["while", "keyword.control"],
    ["break", "keyword.control"],
    ["crash", "keyword.control"],
  ]);

expect("literals",
  `timeout = 30s
drop = 10%
ratio = 0.5f
latency = 5ms..=2s
letter = """
  Dear \${name},\\n
  """`,
  [
    ["30", "constant.numeric"],
    ["s", "keyword.other.unit", 0],
    ["10", "constant.numeric"],
    ["%", "keyword.other.unit"],
    ["0.5", "constant.numeric"],
    ["f", "keyword.other.unit"],
    ["..=", "keyword.operator.range"],
    ["Dear", "string.quoted.triple"],
    ["name", "meta.template.expression"],
  ]);

expect("operators", "a = n / 2\nb = x?.to_int()?\nc = id != other", [
  ["/", "keyword.operator.arithmetic"],
  ["?", "keyword.operator.try"],
  ["!=", "keyword.operator.comparison"],
  ["!=", "!keyword.other.effect"],
]);

expect("comments", `//! The file's doc.
// A line comment.
/// Docs for keeps.
keeps = |s, i| s /* inline */ and i  // trailing
/* outer /* nested */ still a comment */
url = "https://example.com/*not a comment*/"`, [
  ["The file's doc.", "comment.line.documentation"],
  ["A line comment.", "comment.line.double-slash"],
  ["Docs for keeps.", "comment.line.documentation"],
  ["inline", "comment.block"],
  ["trailing", "comment.line.double-slash"],
  ["still a comment", "comment.block"],
  ["and", "!comment"],
  ["url", "!comment"],
]);

expect("where clauses", "largest : List(a) -> Option(a) where [a.compare : a, a -> Int]", [
  ["largest", "entity.name.function"],
  ["where", "keyword.other"],
  ["Option", "support.type.builtin"],
]);

expect("effects and handlers",
  `effect Audit {
  note : Text -> {}
  final abort : Text -> a
}
handler audit_to_table : Audit {
  note = |event| audit_log.insert(AuditEntry { event })
}
run = || with audit_to_table { clear_done(id) }`,
  [
    ["effect", "storage.type.effect"],
    ["Audit", "entity.name.type", 0],
    ["note", "entity.name.function", 0],
    ["final", "storage.modifier.final"],
    ["handler", "storage.type.handler"],
    ["audit_to_table", "entity.name.function"],
    ["with", "keyword.control"],
  ]);

expect("configuration is records",
  `prod = Deployment {
  machines: {
    server: Machine { platform: "weft/server-rust-postgres", count: Many },
  },
  consistency: { items: Session },
  budgets: { add_item: Budget { p99_latency: 50ms } },
}`,
  [
    ["Deployment", "support.type.builtin"],
    ["machines", "variable.other.property"],
    ["Machine", "support.type.builtin"],
    ["Many", "entity.name.type"],
    ["Session", "entity.name.type"],
    ["add_item", "variable.other.property"],
    ["50", "constant.numeric"],
  ]);

expect("markdown code blocks",
  'Some prose.\n\n```weft title="commands/items.weft"\nset_done = |id, done| items.update(id, |i| { ..i, done })\n```\n\nMore prose with record in it.',
  [
    ["set_done", "entity.name.function"],
    ["update", "entity.name.function.member"],
    ["More prose with record in it.", "!storage.type.record"],
  ],
  "markdown");

// ── Coverage ─────────────────────────────────────────────────

console.log("coverage");

function uncovered(code) {
  const misses = [];
  for (const t of rawTokens(hl, code)) {
    const onlyRoot = t.scopes.length === 1 && t.scopes[0] === "source.weft";
    if (onlyRoot && t.text.trim() !== "") misses.push(`line ${t.line}: ${JSON.stringify(t.text.trim())}`);
  }
  return misses;
}

const sources = [
  ...weftFiles().map((p) => ({ name: relative(repoRoot, p), code: readFileSync(p, "utf8") })),
  ...docBlocks().map((b, i) => ({ name: `${b.file} block ${i + 1}`, code: b.code })),
];
for (const { name, code } of sources) {
  const misses = uncovered(code);
  if (misses.length) fail(`${name}: unscoped text\n         ${misses.slice(0, 8).join("\n         ")}`);
}
console.log(`  ${sources.length} sources scanned`);

console.log(failures ? `\n${failures} failure(s)` : "\nall passed");
process.exit(failures ? 1 : 0);
