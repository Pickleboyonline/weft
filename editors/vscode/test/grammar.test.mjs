// Grammar tests. Run with: npm test
//
// 1. Scope checks: specific tokens in real weft snippets get the scope we expect.
// 2. Coverage: every non-space character in every .weft file and every ```weft
//    block in the docs is claimed by some rule (nothing falls through unscoped).
import { highlighter, rawTokens, weftFiles, docBlocks, readFileSync, relative, repoRoot } from "./shared.mjs";

const hl = await highlighter();
let failures = 0;

function fail(msg) {
  failures++;
  console.log(`  FAIL ${msg}`);
}

// expect(code, [text, scope, nth?]...): the nth token whose text is `text`
// must have a scope starting with `scope`. A leading "!" means it must NOT.
function expect(title, code, checks, lang = "weft") {
  const toks = rawTokens(hl, code, lang);
  for (const [text, scope, nth = 0] of checks) {
    const matches = toks.filter((t) => t.text.trim() === text);
    const tok = matches[nth];
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

expect("command header", "command create_list!(name: Text) {", [
  ["command", "storage.type.command"],
  ["create_list", "entity.name.function"],
  ["!", "keyword.other.effect"],
  ["name", "variable.other.property"],
  ["Text", "support.type.builtin"],
]);

expect("reject", "  if cleaned.is_empty() { reject EmptyName }", [
  ["if", "keyword.control"],
  ["is_empty", "entity.name.function.member"],
  ["reject", "keyword.control"],
  ["EmptyName", "entity.name.type"],
]);

expect("effectful method", "  lists.insert!(TodoList { name: cleaned })", [
  ["insert", "entity.name.function.member"],
  ["!", "keyword.other.effect"],
  ["TodoList", "entity.name.type"],
]);

expect("view", "view items_in(list: Id, show: Filter) -> List(Item) =", [
  ["view", "storage.type.view"],
  ["items_in", "entity.name.function"],
  ["->", "keyword.operator.arrow"],
  ["List", "support.type.builtin"],
]);

expect("accessor shorthand", "    .sort_by(.created)", [
  ["sort_by", "entity.name.function.member"],
  ["created", "variable.other.property"],
]);

expect("lambda definition", "  update = |m, msg| match msg {", [
  ["update", "entity.name.function"],
  ["m", "variable.parameter"],
  ["msg", "variable.parameter"],
  ["match", "keyword.control"],
]);

expect("markup and interpolation", '    span(class: "count") { "${row.open}" }', [
  ["span", "entity.name.tag"],
  ["class", "variable.other.property"],
  ["count", "string.quoted.double"],
  ["row", "meta.template.expression"],
  ["open", "variable.other.property"],
]);

expect("comment swallows keywords", '# command x!() "not a string"', [
  ['command x!() "not a string"', "comment.line.number-sign"],
  ['command x!() "not a string"', "!storage.type.command"],
]);

expect("record", "record Item {\n  key id:  Id   = new_id!()\n  done:    Bool = False\n}", [
  ["record", "storage.type.record"],
  ["Item", "entity.name.type"],
  ["key", "storage.modifier.key"],
  ["id", "variable.other.property"],
  ["new_id", "entity.name.function"],
  ["!", "keyword.other.effect"],
  ["False", "constant.language.boolean"],
]);

expect("table", "table items : Item", [
  ["table", "storage.type.table"],
  ["items", "entity.name.variable.table"],
  ["Item", "entity.name.type"],
]);

expect("type declaration and fields", "  Model : {\n    selected:   [None, Some(Id)],\n  }", [
  ["Model", "entity.name.type"],
  ["selected", "variable.other.property"],
  ["Some", "entity.name.type"],
]);

expect("for loop", "for row in all_lists() key row.list.id {", [
  ["for", "keyword.control"],
  ["in", "keyword.control"],
  ["key", "keyword.control"],
  ["all_lists", "entity.name.function"],
  ["list", "variable.other.property"],
]);

expect("words", "  Active -> not i.done\n  None -> (m, none)\n  x = a and b or c", [
  ["not", "keyword.operator.wordlike"],
  ["none", "constant.language"],
  ["and", "keyword.operator.wordlike"],
  ["or", "keyword.operator.wordlike"],
]);

expect("do ... else", "  CreateList -> ({ ..m, list_draft: \"\" }, do create_list!(m.list_draft) else Rejected)", [
  ["..", "keyword.operator.spread"],
  ["do", "keyword.control"],
  ["create_list", "entity.name.function"],
  ["!", "keyword.other.effect"],
  ["else", "keyword.control"],
]);

expect("not-equals is not an effect", "if id != x { }", [
  ["!=", "keyword.operator.comparison"],
  ["!=", "!keyword.other.effect"],
]);

expect("signatures with effect rows",
  "add_todo!   : Text => {}  ! <write todos, reject [EmptyTitle], random, time>\nApp.view    : Model -> Html(Msg)    ! <read todo_list, remaining>",
  [
    ["add_todo", "entity.name.function"],
    ["=>", "keyword.other.effect.arrow"],
    ["write", "keyword.other.effect.kind"],
    ["todos", "meta.effects"],
    ["random", "keyword.other.effect.kind"],
    ["App.view", "entity.name.function"],
    ["read", "keyword.other.effect.kind"],
  ]);

expect("extern",
  'extern send_email! : Text, Text => {} ! <reject [Bounced]>\n  needs email\n  sim { ok 95%, else Bounced, latency 50ms..2s }',
  [
    ["extern", "storage.type.extern"],
    ["send_email", "entity.name.function"],
    ["email", "constant.language.capability"],
    ["sim", "storage.type.sim"],
    ["ok", "constant.language.deploy"],
    ["95", "constant.numeric"],
    ["%", "keyword.other.unit"],
    ["latency", "constant.language.deploy"],
    ["ms", "keyword.other.unit"],
  ]);

expect("effects and handlers",
  "effect Audit {\n  fun note(event: Text)\n}\nhandler audit_table for Audit {\n  final ctl stop(why: Text)\n}",
  [
    ["effect", "storage.type.effect"],
    ["Audit", "entity.name.type"],
    ["fun", "storage.type.function"],
    ["note", "entity.name.function"],
    ["handler", "storage.type.handler"],
    ["audit_table", "entity.name.function"],
    ["for", "keyword.other.for"],
    ["final", "storage.modifier.final"],
    ["ctl", "storage.type.function"],
  ]);

const deploy = `deploy prod {
  machine server  on "weft/server-rust-postgres"
  machine browser on "weft/browser-ts" { count: many, offline: True }
  place lists, items on server
  consistency lists, items: session
  budget daily_active: max $50/month
  pin daily_active:    materialized
}

simulate flaky_wifi {
  network:  drop 10%, latency 5ms..2s, offline 30%
  steps:    500 random msgs per client
}

pin = 3`;

expect("deploy blocks", deploy, [
  ["deploy", "storage.type.deploy"],
  ["prod", "entity.name.namespace"],
  ["machine", "keyword.other.deploy"],
  ["server", "entity.name.variable.machine"],
  ["weft/server-rust-postgres", "string.quoted.double"],
  ["many", "constant.language.deploy"],
  ["place", "keyword.other.deploy"],
  ["consistency", "keyword.other.deploy"],
  ["session", "constant.language.deploy"],
  ["budget", "keyword.other.deploy"],
  ["$50/", "constant.numeric.money"],
  ["month", "keyword.other.unit"],
  ["materialized", "constant.language.deploy"],
  ["simulate", "storage.type.deploy"],
  ["10", "constant.numeric"],
  ["..", "keyword.operator.spread"],
  ["msgs", "constant.language.deploy"],
  // Deploy vocabulary is not highlighted outside deploy blocks.
  ["pin", "keyword.other.deploy", 0],
  ["pin", "!keyword.other.deploy", 1],
  ["pin", "variable.other.definition", 1],
]);

{
  // The block that holds { count: many, ... } must not end the deploy block early,
  // and a second block must not nest inside the first.
  const toks = rawTokens(hl, deploy);
  const place = toks.find((t) => t.text.trim() === "place");
  if (!place?.scopes.includes("meta.block.deploy.weft")) fail("nested braces ended the deploy block early");
  const sim = toks.find((t) => t.text.trim() === "simulate");
  if (sim?.scopes.filter((s) => s === "meta.block.deploy.weft").length !== 1) fail("simulate block nested inside deploy block");
}

expect("markdown code blocks",
  "Some prose.\n\n```weft\ncommand toggle!(id: Id) {\n  todos.update!(id, |t| { ..t, done: not t.done })\n}\n```\n\nMore prose with command in it.",
  [
    ["command", "storage.type.command", 0],
    ["toggle", "entity.name.function"],
    ["update", "entity.name.function.member"],
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
