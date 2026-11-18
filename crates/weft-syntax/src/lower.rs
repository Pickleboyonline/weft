//! Turns the tree-sitter concrete tree into the typed AST.

use crate::ast::*;
use crate::diag::Diagnostic;
use tree_sitter::Node;

pub struct Lowerer<'a> {
    src: &'a str,
    pub diags: Vec<Diagnostic>,
}

fn span(n: Node) -> Span {
    let p = n.start_position();
    Span { start: n.start_byte(), end: n.end_byte(), line: p.row, col: p.column }
}

fn named_children(n: Node) -> Vec<Node> {
    let mut c = n.walk();
    n.named_children(&mut c).filter(|k| !is_comment(*k)).collect()
}

fn is_comment(n: Node) -> bool {
    matches!(n.kind(), "line_comment" | "block_comment")
}

impl<'a> Lowerer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lowerer { src, diags: Vec::new() }
    }

    fn text(&self, n: Node) -> &'a str {
        &self.src[n.byte_range()]
    }

    fn ident(&self, n: Node) -> Ident {
        Ident { name: self.text(n).to_string(), span: span(n) }
    }

    fn field_ident(&self, n: Node, f: &str) -> Ident {
        match n.child_by_field_name(f) {
            Some(c) => self.ident(c),
            None => Ident { name: String::new(), span: span(n) },
        }
    }

    fn err(&mut self, n: Node, code: &'static str, msg: impl Into<String>) {
        self.diags.push(Diagnostic::error(code, msg, span(n)));
    }

    pub fn file(&mut self, root: Node) -> File {
        let mut items = Vec::new();
        for c in named_children(root) {
            if let Some(i) = self.item(c) {
                items.push(i);
            }
        }
        File { items }
    }

    fn item(&mut self, n: Node) -> Option<Item> {
        let kind = match n.kind() {
            "record_declaration" => {
                let name = self.field_ident(n, "name");
                let mut fields = Vec::new();
                let mut methods = Vec::new();
                for c in named_children(n).into_iter().skip(1) {
                    match c.kind() {
                        "record_field" => fields.push(RecordField {
                            key: c.child_by_field_name("key").is_some(),
                            name: self.field_ident(c, "name"),
                            ty: self.ty_field(c, "type"),
                            default: c.child_by_field_name("default").map(|d| self.expr(d)),
                        }),
                        "binding" => methods.push(self.binding(c)),
                        _ => {}
                    }
                }
                ItemKind::Record(Record { name, fields, methods })
            }
            "component_declaration" => {
                let name = self.field_ident(n, "name");
                let mut params = Vec::new();
                if let Some(pl) = n.child_by_field_name("parameters") {
                    for p in named_children(pl) {
                        params.push(ComponentParam {
                            name: self.field_ident(p, "name"),
                            ty: self.ty_field(p, "type"),
                            default: p.child_by_field_name("default").map(|d| self.expr(d)),
                        });
                    }
                }
                let body = self.block(n.child_by_field_name("body")?);
                ItemKind::Component(Component { name, params, body })
            }
            "effect_declaration" => {
                let name = self.field_ident(n, "name");
                let ops = named_children(n)
                    .into_iter()
                    .filter(|c| c.kind() == "operation")
                    .map(|o| Operation {
                        kind: match o.child_by_field_name("kind").map(|k| self.text(k)) {
                            Some("final") => OpKind::Final,
                            Some("ctl") => OpKind::Control,
                            _ => OpKind::Resume,
                        },
                        name: self.field_ident(o, "name"),
                        ty: self.ty_field(o, "type"),
                    })
                    .collect();
                ItemKind::Effect(EffectDecl { name, ops })
            }
            "handler_declaration" => ItemKind::Handler(HandlerDecl {
                name: self.field_ident(n, "name"),
                effect: self.field_ident(n, "effect"),
                body: self.block(n.child_by_field_name("body")?),
            }),
            "signature" => ItemKind::Signature(self.signature(n)),
            "alias" => {
                let name = self.field_ident(n, "name");
                let params = named_children(n)
                    .into_iter()
                    .filter(|c| c.kind() == "identifier")
                    .map(|c| self.ident(c))
                    .collect();
                ItemKind::Alias(Alias { name, params, ty: self.ty_field(n, "type") })
            }
            "binding" => ItemKind::Binding(self.binding(n)),
            "expect" => ItemKind::Expect(self.expr(n.child_by_field_name("condition")?)),
            "ERROR" => return None,
            other => {
                self.err(n, "syntax", format!("unexpected `{other}` at the top level"));
                return None;
            }
        };
        Some(Item { kind, span: span(n) })
    }

    fn signature(&mut self, n: Node) -> Signature {
        let constraints = n
            .child_by_field_name("where")
            .map(|w| {
                named_children(w)
                    .into_iter()
                    .map(|c| Constraint {
                        var: self.field_ident(c, "variable"),
                        method: self.field_ident(c, "method"),
                        ty: self.ty_field(c, "type"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Signature { name: self.field_ident(n, "name"), ty: self.ty_field(n, "type"), constraints }
    }

    fn binding(&mut self, n: Node) -> Binding {
        let left = match n.child_by_field_name("left") {
            Some(l) => Pattern::Name(self.ident(l)),
            None => Pattern::Error(span(n)),
        };
        let value = self.expr_field(n, "value");
        Binding { left, value, span: span(n) }
    }

    pub fn block(&mut self, n: Node) -> Block {
        let stmts = named_children(n).into_iter().filter_map(|c| self.stmt(c)).collect();
        Block { stmts, span: span(n) }
    }

    fn stmt(&mut self, n: Node) -> Option<Stmt> {
        let kind = match n.kind() {
            "binding" => StmtKind::Binding(self.binding(n)),
            "let_declaration" => StmtKind::Let {
                pattern: n.child_by_field_name("pattern").map(|p| self.pattern(p)).unwrap_or(Pattern::Error(span(n))),
                mutable: n.child_by_field_name("mutable").is_some(),
                value: self.expr_field(n, "value"),
                otherwise: n.child_by_field_name("alternative").map(|b| self.block(b)),
            },
            "state_declaration" => StmtKind::State {
                name: self.field_ident(n, "name"),
                ty: n.child_by_field_name("type").map(|t| self.ty(t)),
                value: self.expr_field(n, "value"),
            },
            "style_block" => StmtKind::Style,
            "for_statement" => StmtKind::For {
                pattern: self.pattern(n.child_by_field_name("pattern")?),
                iterable: self.expr_field(n, "iterable"),
                body: self.block(n.child_by_field_name("body")?),
            },
            "while_statement" => StmtKind::While {
                condition: self.expr_field(n, "condition"),
                body: self.block(n.child_by_field_name("body")?),
            },
            "break_statement" => StmtKind::Break,
            "continue_statement" => StmtKind::Continue,
            "return_statement" => StmtKind::Return(named_children(n).first().map(|e| self.expr(*e))),
            "with_statement" => StmtKind::With {
                handler: self.expr_field(n, "handler"),
                body: self.block(n.child_by_field_name("body")?),
            },
            "defer_statement" => StmtKind::Defer(self.block(n.child_by_field_name("body")?)),
            "ERROR" => return None,
            _ => StmtKind::Expr(self.expr(n)),
        };
        Some(Stmt { kind, span: span(n) })
    }

    fn expr_field(&mut self, n: Node, f: &str) -> Expr {
        match n.child_by_field_name(f) {
            Some(c) => self.expr(c),
            None => Expr { kind: ExprKind::Error, span: span(n) },
        }
    }

    fn boxed(&mut self, n: Node, f: &str) -> Box<Expr> {
        Box::new(self.expr_field(n, f))
    }

    pub fn expr(&mut self, n: Node) -> Expr {
        let sp = span(n);
        let kind = match n.kind() {
            "identifier" => ExprKind::Name(self.text(n).to_string()),
            "type_identifier" => ExprKind::Tag(self.text(n).to_string()),
            "number" => {
                let t = self.text(n);
                ExprKind::Number { text: t.to_string(), kind: number_kind(t) }
            }
            "text" => ExprKind::Text(self.text_parts(n)),
            "list" => ExprKind::List(named_children(n).into_iter().map(|c| self.expr(c)).collect()),
            "record" => ExprKind::Record(self.fields(n)),
            "typed_record" => ExprKind::TypedRecord { ty: self.field_ident(n, "type"), fields: self.fields(n) },
            "block" => ExprKind::Block(self.block(n)),
            "lambda" => {
                let params = named_children(n)
                    .into_iter()
                    .filter(|c| c.kind() == "parameter")
                    .map(|p| LambdaParam {
                        pattern: p.child_by_field_name("pattern").map(|x| self.pattern(x)).unwrap_or(Pattern::Error(span(p))),
                        ty: p.child_by_field_name("type").map(|t| self.ty(t)),
                    })
                    .collect();
                let result = n.child_by_field_name("result").map(|t| self.ty(t));
                ExprKind::Lambda(Lambda { params, result, body: self.boxed(n, "body") })
            }
            "assignment" => ExprKind::Assign { name: self.field_ident(n, "left"), value: self.boxed(n, "value") },
            "if_expression" => {
                let cond = n.child_by_field_name("condition");
                let (pattern, condition) = match cond {
                    Some(c) if c.kind() == "let_condition" => (
                        c.child_by_field_name("pattern").map(|p| Box::new(self.pattern(p))),
                        Box::new(self.expr_field(c, "value")),
                    ),
                    _ => (None, self.boxed(n, "condition")),
                };
                ExprKind::If {
                pattern,
                condition,
                then: match n.child_by_field_name("consequence") {
                    Some(b) => self.block(b),
                    None => Block { stmts: vec![], span: sp },
                },
                otherwise: n.child_by_field_name("alternative").map(|a| Box::new(self.expr(a))),
            }
            }
            "match_expression" => {
                let value = self.boxed(n, "value");
                let arms = named_children(n)
                    .into_iter()
                    .filter(|c| c.kind() == "match_arm")
                    .map(|a| Arm {
                        pattern: a.child_by_field_name("pattern").map(|p| self.pattern(p)).unwrap_or(Pattern::Error(span(a))),
                        guard: a.child_by_field_name("guard").map(|g| self.expr(g)),
                        value: self.expr_field(a, "value"),
                    })
                    .collect();
                ExprKind::Match { value, arms }
            }
            "accessor" => ExprKind::Accessor(self.field_ident(n, "field")),
            "parenthesized" => return named_children(n).first().map(|c| self.expr(*c)).unwrap_or(Expr { kind: ExprKind::Error, span: sp }),
            "unit" => ExprKind::Unit,
            "call" => {
                let function = self.boxed(n, "function");
                let args = n.child_by_field_name("arguments").map(|a| self.args(a)).unwrap_or_default();
                ExprKind::Call { function, args }
            }
            "element" => {
                let head = self.boxed(n, "name");
                let children = match n.child_by_field_name("children") {
                    Some(b) => self.block(b),
                    None => Block { stmts: vec![], span: sp },
                };
                ExprKind::Element { head, children }
            }
            "field_access" => ExprKind::Field { value: self.boxed(n, "value"), field: self.field_ident(n, "field") },
            "await_expression" => ExprKind::Await(self.boxed(n, "value")),
            "try_expression" => ExprKind::Try(self.boxed(n, "value")),
            "unary_expression" => {
                let op = match n.child_by_field_name("operator").map(|o| self.text(o)) {
                    Some("not") => UnOp::Not,
                    _ => UnOp::Neg,
                };
                ExprKind::Unary { op, operand: self.boxed(n, "operand") }
            }
            "binary_expression" | "range_expression" => {
                let op = match n.child_by_field_name("operator").map(|o| self.text(o)).unwrap_or("") {
                    "or" => BinOp::Or,
                    "and" => BinOp::And,
                    "==" => BinOp::Eq,
                    "!=" => BinOp::Ne,
                    "<" => BinOp::Lt,
                    "<=" => BinOp::Le,
                    ">" => BinOp::Gt,
                    ">=" => BinOp::Ge,
                    "+" => BinOp::Add,
                    "-" => BinOp::Sub,
                    "*" => BinOp::Mul,
                    "/" => BinOp::Div,
                    "%" => BinOp::Rem,
                    "..<" => BinOp::RangeExclusive,
                    _ => BinOp::RangeInclusive,
                };
                let (l, r) = if n.kind() == "range_expression" { ("start", "end") } else { ("left", "right") };
                let left = self.boxed(n, l);
                let right = self.boxed(n, r);
                if op.is_comparison() {
                    let chained = |e: &Expr| matches!(&e.kind, ExprKind::Binary { op, .. } if op.is_comparison());
                    if chained(&left) || chained(&right) {
                        self.err(n, "chained-comparison", "comparisons can't be chained: write `a < b and b < c`");
                    }
                }
                ExprKind::Binary { op, left, right }
            }
            "crash_expression" => ExprKind::Crash(Box::new(
                named_children(n).first().map(|c| self.expr(*c)).unwrap_or(Expr { kind: ExprKind::Error, span: sp }),
            )),
            "ERROR" => ExprKind::Error,
            other => {
                self.err(n, "syntax", format!("unexpected `{other}`"));
                ExprKind::Error
            }
        };
        Expr { kind, span: sp }
    }

    fn text_parts(&mut self, n: Node) -> Vec<TextPart> {
        let mut parts = Vec::new();
        for c in named_children(n) {
            match c.kind() {
                "interpolation" => {
                    if let Some(e) = named_children(c).first() {
                        parts.push(TextPart::Interpolation(self.expr(*e)));
                    }
                }
                _ => parts.push(TextPart::Literal(self.text(c).to_string())),
            }
        }
        parts
    }

    fn fields(&mut self, n: Node) -> Vec<Field> {
        let mut out = Vec::new();
        for c in named_children(n) {
            match c.kind() {
                "field" => out.push(Field::Named { name: self.field_ident(c, "name"), value: self.expr_field(c, "value") }),
                "shorthand_field" => out.push(Field::Shorthand(self.field_ident(c, "name"))),
                "spread" => out.push(Field::Spread(self.expr_field(c, "value"))),
                _ => {}
            }
        }
        out
    }

    fn args(&mut self, n: Node) -> Vec<Arg> {
        named_children(n)
            .into_iter()
            .filter(|c| c.kind() == "argument")
            .map(|a| Arg {
                name: a.child_by_field_name("name").map(|x| self.ident(x)),
                value: self.expr_field(a, "value"),
            })
            .collect()
    }

    fn pattern(&mut self, n: Node) -> Pattern {
        match n.kind() {
            "identifier" if self.text(n) == "_" => Pattern::Wildcard(span(n)),
            "identifier" => Pattern::Name(self.ident(n)),
            "type_identifier" => Pattern::Tag { name: self.ident(n), args: vec![] },
            "tag_pattern" => {
                let name = self.field_ident(n, "tag");
                let args = named_children(n).into_iter().skip(1).map(|c| self.pattern(c)).collect();
                Pattern::Tag { name, args }
            }
            "record_pattern" => {
                let mut fields = Vec::new();
                let mut cur = n.walk();
                let mut pending: Option<Ident> = None;
                if cur.goto_first_child() {
                    loop {
                        let c = cur.node();
                        if c.is_named() && !is_comment(c) {
                            match cur.field_name() {
                                Some("name") => pending = Some(self.ident(c)),
                                Some("pattern") => {
                                    let name = pending.take().unwrap_or(Ident { name: String::new(), span: span(c) });
                                    fields.push((name, Some(self.pattern(c))));
                                }
                                _ => fields.push((self.ident(c), None)),
                            }
                        }
                        if !cur.goto_next_sibling() {
                            break;
                        }
                    }
                }
                Pattern::Record { fields, span: span(n) }
            }
            "list_pattern" => {
                let mut items = Vec::new();
                let mut rest = None;
                for c in named_children(n) {
                    if c.kind() == "rest_pattern" {
                        rest = Some(named_children(c).first().map(|r| self.ident(*r)));
                    } else {
                        items.push(self.pattern(c));
                    }
                }
                Pattern::List { items, rest, span: span(n) }
            }
            "number" | "text" => Pattern::Literal(self.expr(n)),
            "or_pattern" => Pattern::Or(named_children(n).into_iter().map(|c| self.pattern(c)).collect(), span(n)),
            _ => Pattern::Error(span(n)),
        }
    }

    fn ty_field(&mut self, n: Node, f: &str) -> Type {
        match n.child_by_field_name(f) {
            Some(t) => self.ty(t),
            None => Type::Error(span(n)),
        }
    }

    pub fn ty(&mut self, n: Node) -> Type {
        let sp = span(n);
        match n.kind() {
            "type_identifier" => Type::Name { name: self.ident(n), args: vec![] },
            "type_application" => {
                let name = self.field_ident(n, "name");
                let args = named_children(n).into_iter().skip(1).map(|c| self.ty(c)).collect();
                Type::Name { name, args }
            }
            "type_variable" => Type::Var(Ident { name: self.text(n).to_string(), span: sp }),
            "function_type" => {
                let kids = named_children(n);
                let (params, result) = kids.split_at(kids.len().saturating_sub(1));
                Type::Fn {
                    params: params.iter().map(|p| self.ty(*p)).collect(),
                    result: Box::new(result.first().map(|r| self.ty(*r)).unwrap_or(Type::Error(sp))),
                    span: sp,
                }
            }
            "record_type" => {
                let kids = named_children(n);
                let fields = kids.chunks(2).filter(|c| c.len() == 2).map(|c| (self.ident(c[0]), self.ty(c[1]))).collect();
                Type::Record { fields, span: sp }
            }
            "tag_union_type" => {
                let mut tags = Vec::new();
                let mut open = false;
                let mut cur = n.walk();
                for c in n.children(&mut cur) {
                    match c.kind() {
                        "type_identifier" => tags.push((self.ident(c), vec![])),
                        "tag_type" => {
                            let kids = named_children(c);
                            let name = self.ident(kids[0]);
                            let args = kids[1..].iter().map(|k| self.ty(*k)).collect();
                            tags.push((name, args));
                        }
                        ".." => open = true,
                        _ => {}
                    }
                }
                Type::Tags { tags, open, span: sp }
            }
            "unit_type" => Type::Unit(sp),
            _ => Type::Error(sp),
        }
    }
}

fn number_kind(t: &str) -> NumberKind {
    if t.ends_with('%') {
        NumberKind::Percent
    } else if t.ends_with('f') {
        NumberKind::Float
    } else if t.ends_with("KB") || t.ends_with("MB") || t.ends_with("GB") || t.ends_with("TB") || t.ends_with('B') {
        NumberKind::Size
    } else if t.chars().last().is_some_and(|c| c.is_ascii_alphabetic()) {
        NumberKind::Duration
    } else if t.contains('.') {
        NumberKind::Dec
    } else {
        NumberKind::Int
    }
}
