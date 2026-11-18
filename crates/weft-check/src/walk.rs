//! One pass over every body: resolves names, enforces scoping, and records
//! each unit's direct effects and calls. Rules that need whole-program effects
//! run afterwards (see `rules`).

use std::collections::{BTreeMap, HashMap, HashSet};

use weft_syntax::ast::*;
use weft_syntax::Diagnostic;

use crate::builtins;
use crate::effects::Effect;
use crate::program::{is_task, Def, DefKind, Globals, Source, TableScope};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Function,
    Stateful,
    Constant,
    Query,
    Command,
    Check,
    Expect,
    View,
    Handler,
    Task,
    Method,
    HandlerImpl,
}

#[derive(Debug)]
pub struct Unit {
    pub kind: UnitKind,
    pub name: String,
    /// The component a view, handler or task belongs to.
    pub owner: Option<String>,
    pub file: usize,
    pub span: Span,
    pub client: bool,
    pub direct: BTreeMap<Effect, Span>,
    pub calls: Vec<(String, Span)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindKind {
    Let,
    Param,
    Var { owner: usize },
    State,
    Global,
    /// A snippet binding of a name the snippet doesn't declare: probably a
    /// component's `state` defined elsewhere, so it may be assigned.
    Unknown,
}

#[derive(Clone, Copy)]
struct Binder {
    kind: BindKind,
    /// Where it was declared, for "bound here" labels.
    span: Option<Span>,
}

pub struct Walker<'a> {
    g: &'a Globals,
    sources: &'a [Source],
    strict: bool,
    pub diags: Vec<(usize, Diagnostic)>,
    pub units: Vec<Unit>,
    scopes: Vec<HashMap<String, Binder>>,
    file: usize,
    unit: usize,
    lambda_id: usize,
    next_lambda: usize,
    client: bool,
    component: Option<String>,
    /// True while walking statements at the top of a component or stateful
    /// function body, where stateful functions may be called.
    top_of_body: bool,
}

impl<'a> Walker<'a> {
    pub fn new(g: &'a Globals, sources: &'a [Source], strict: bool) -> Self {
        Walker {
            g,
            sources,
            strict,
            diags: Vec::new(),
            units: Vec::new(),
            scopes: Vec::new(),
            file: 0,
            unit: 0,
            lambda_id: 0,
            next_lambda: 1,
            client: false,
            component: None,
            top_of_body: false,
        }
    }

    fn error(&mut self, code: &'static str, msg: impl Into<String>, span: Span) {
        self.diags.push((self.file, Diagnostic::error(code, msg, span)));
    }

    fn new_unit(&mut self, kind: UnitKind, name: impl Into<String>, span: Span, client: bool) -> usize {
        self.units.push(Unit {
            kind,
            name: name.into(),
            owner: self.component.clone(),
            file: self.file,
            span,
            client,
            direct: BTreeMap::new(),
            calls: Vec::new(),
        });
        self.units.len() - 1
    }

    fn effect(&mut self, e: Effect, span: Span) {
        self.units[self.unit].direct.entry(e).or_insert(span);
    }

    // ── Scopes ────────────────────────────────────────────────

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
    }

    fn lookup(&self, name: &str) -> Option<Binder> {
        for s in self.scopes.iter().rev() {
            if let Some(b) = s.get(name) {
                return Some(*b);
            }
        }
        if self.g.values.contains_key(name) {
            return Some(Binder { kind: BindKind::Global, span: None });
        }
        None
    }

    fn declare(&mut self, id: &Ident, kind: BindKind) {
        if id.name == "_" || id.name.is_empty() {
            return;
        }
        // A `let` or a parameter may shadow any name: `let` makes it visible.
        if let Some(s) = self.scopes.last_mut() {
            s.insert(id.name.clone(), Binder { kind, span: Some(id.span) });
        }
    }

    /// Declares without the shadowing check (component members are declared
    /// up front, in any order).
    fn predeclare(&mut self, name: &str, kind: BindKind) {
        if let Some(s) = self.scopes.last_mut() {
            s.insert(name.to_string(), Binder { kind, span: None });
        }
    }

    fn resolve_value(&mut self, name: &str, span: Span) {
        if self.lookup(name).is_some()
            || builtins::FUNCTIONS.contains(&name)
            || builtins::MODULES.contains(&name)
            || builtins::MARKUP_CONSTANTS.contains(&name)
            || name == "_"
        {
            return;
        }
        if self.strict {
            self.error("unknown-name", format!("unknown name `{name}`"), span);
        }
    }

    fn global(&self, name: &str) -> Option<&'a Def> {
        if self.scopes.iter().any(|s| s.contains_key(name)) {
            return None;
        }
        self.g.values.get(name)
    }

    // ── Items ─────────────────────────────────────────────────

    pub fn walk_program(&mut self) {
        for fi in 0..self.sources.len() {
            self.file = fi;
            let items = &self.sources[fi].file.items;
            for item in items {
                self.item(item);
            }
        }
    }

    fn item(&mut self, item: &'a Item) {
        self.component = None;
        self.client = false;
        self.lambda_id = 0;
        match &item.kind {
            ItemKind::Binding(b) => {
                let name = match &b.left {
                    Pattern::Name(n) => n.name.clone(),
                    _ => return,
                };
                let def = self.g.values.get(&name).map(|d| d.kind.clone());
                match def {
                    _ if !self.strict && name == "view" => {
                        // A snippet's `view = …` belongs to a component.
                        self.client = true;
                        self.unit = self.new_unit(UnitKind::View, "view", b.left.span(), true);
                        self.expr(&b.value);
                    }
                    Some(DefKind::Function { stateful }) => {
                        let kind = if stateful { UnitKind::Stateful } else { UnitKind::Function };
                        self.unit = self.new_unit(kind, &name, b.left.span(), stateful);
                        self.client = stateful;
                        self.top_of_body = stateful;
                        if let ExprKind::Lambda(l) = &b.value.kind {
                            self.lambda(l, true);
                        }
                    }
                    Some(DefKind::Query { .. }) | Some(DefKind::Command { .. }) | Some(DefKind::Check) => {
                        let kind = match def {
                            Some(DefKind::Query { .. }) => UnitKind::Query,
                            Some(DefKind::Command { .. }) => UnitKind::Command,
                            _ => UnitKind::Check,
                        };
                        self.unit = self.new_unit(kind, &name, b.left.span(), false);
                        if let ExprKind::Call { args, .. } = &b.value.kind {
                            for a in args {
                                match &a.value.kind {
                                    ExprKind::Lambda(l) => {
                                        if kind != UnitKind::Check {
                                            self.require_annotations(&name, kind, l, a.value.span);
                                        }
                                        self.lambda(l, true);
                                    }
                                    _ => self.expr(&a.value),
                                }
                            }
                        }
                    }
                    _ => {
                        self.unit = self.new_unit(UnitKind::Constant, &name, b.left.span(), false);
                        self.expr(&b.value);
                    }
                }
            }
            ItemKind::Expect(e) => {
                self.unit = self.new_unit(UnitKind::Expect, "expect", e.span, false);
                self.expr(e);
            }
            ItemKind::Record(r) => {
                self.unit = self.new_unit(UnitKind::Method, format!("{} defaults", r.name.name), r.name.span, false);
                for f in &r.fields {
                    if let Some(d) = &f.default {
                        self.expr(d);
                    }
                }
                for m in &r.methods {
                    if let Pattern::Name(n) = &m.left {
                        self.unit = self.new_unit(UnitKind::Method, format!("{}.{}", r.name.name, n.name), n.span, false);
                        self.expr(&m.value);
                    }
                }
            }
            ItemKind::Component(c) => self.component(c),
            ItemKind::Handler(h) => {
                for s in &h.body.stmts {
                    if let StmtKind::Binding(b) = &s.kind {
                        if let Pattern::Name(n) = &b.left {
                            self.unit =
                                self.new_unit(UnitKind::HandlerImpl, format!("{}.{}", h.name.name, n.name), n.span, false);
                            self.expr(&b.value);
                        }
                    }
                }
            }
            ItemKind::Effect(_) | ItemKind::Alias(_) | ItemKind::Signature(_) => {}
        }
    }

    fn require_annotations(&mut self, name: &str, kind: UnitKind, l: &Lambda, span: Span) {
        let what = if kind == UnitKind::Query { "Query" } else { "Command" };
        for p in &l.params {
            if p.ty.is_none() {
                self.error(
                    "untyped-remote",
                    format!(
                        "`{name}` is a {what}: give `{}` a type, e.g. `|{}: Text|`; its types are the API",
                        p.pattern.names().first().map(|n| n.name.as_str()).unwrap_or("_"),
                        p.pattern.names().first().map(|n| n.name.as_str()).unwrap_or("x"),
                    ),
                    p.pattern.span(),
                );
            }
        }
        if l.result.is_none() {
            self.error(
                "untyped-remote",
                format!("`{name}` is a {what}: write its result type after the parameters, e.g. `|…| -> List(Item) {{ … }}`"),
                span,
            );
        }
    }

    fn component(&mut self, c: &'a Component) {
        self.component = Some(c.name.name.clone());
        self.client = true;
        let view = self.new_unit(UnitKind::View, format!("{}.view", c.name.name), c.name.span, true);
        self.push();
        for p in &c.params {
            self.predeclare(&p.name.name, BindKind::Param);
            if let Some(d) = &p.default {
                self.unit = view;
                self.expr(d);
            }
        }
        for s in &c.body.stmts {
            match &s.kind {
                StmtKind::State { name, .. } => self.predeclare(&name.name, BindKind::State),
                StmtKind::Binding(b) => {
                    for n in b.left.names() {
                        self.predeclare(&n.name, BindKind::Let);
                    }
                }
                _ => {}
            }
        }
        for s in &c.body.stmts {
            self.unit = view;
            self.lambda_id = 0;
            self.top_of_body = true;
            match &s.kind {
                StmtKind::State { value, .. } => self.expr(value),
                StmtKind::Binding(b) => match (&b.left, &b.value.kind) {
                    (Pattern::Name(n), ExprKind::Lambda(l)) => {
                        self.unit = self.new_unit(UnitKind::Handler, format!("{}.{}", c.name.name, n.name), n.span, true);
                        self.top_of_body = false;
                        self.lambda(l, false);
                    }
                    _ => self.expr(&b.value),
                },
                StmtKind::Expr(e) if is_task(e) => self.task(e),
                StmtKind::Style => {}
                _ => self.stmt(s),
            }
        }
        self.pop();
        self.component = None;
        self.client = false;
    }

    fn task(&mut self, e: &'a Expr) {
        if let ExprKind::Element { head, children } = &e.kind {
            if let ExprKind::Call { args, .. } = &head.kind {
                for a in args {
                    self.expr(&a.value);
                }
            }
            let owner = self.component.clone().unwrap_or_else(|| self.units[self.unit].name.clone());
            let saved = (self.unit, self.top_of_body, self.lambda_id);
            self.unit = self.new_unit(UnitKind::Task, format!("{owner} task"), e.span, true);
            self.top_of_body = false;
            self.lambda_id = self.next_lambda;
            self.next_lambda += 1;
            self.block(children);
            (self.unit, self.top_of_body, self.lambda_id) = saved;
        }
    }

    // ── Lambdas ───────────────────────────────────────────────

    /// Walks a lambda in the current unit with its own scope and identity.
    fn lambda(&mut self, l: &'a Lambda, top: bool) {
        let saved = (self.lambda_id, self.top_of_body);
        self.lambda_id = self.next_lambda;
        self.next_lambda += 1;
        self.push();
        for p in &l.params {
            self.pattern_bind(&p.pattern, BindKind::Param);
        }
        self.top_of_body = top && self.top_of_body;
        match &l.body.kind {
            ExprKind::Block(b) => self.block_stmts(b),
            _ => {
                // A handler's value is thrown away, so a Result it returns
                // is ignored.
                if self.client {
                    self.must_use(&l.body);
                }
                self.expr_or_assign(&l.body)
            }
        }
        self.pop();
        (self.lambda_id, self.top_of_body) = saved;
    }

    /// A lambda that isn't run where it's written (a handler passed as an
    /// attribute, a function stored in a binding): its effects are its own.
    fn deferred_lambda(&mut self, l: &'a Lambda, span: Span) {
        let saved = (self.unit, self.top_of_body);
        let kind = if self.client { UnitKind::Handler } else { UnitKind::Function };
        let owner = self.component.clone().unwrap_or_else(|| self.units[self.unit].name.clone());
        let line = span.line + 1;
        self.unit = self.new_unit(kind, format!("{owner}: handler on line {line}"), span, self.client);
        self.top_of_body = false;
        self.lambda(l, false);
        (self.unit, self.top_of_body) = saved;
    }

    // ── Statements ────────────────────────────────────────────

    fn block(&mut self, b: &'a Block) {
        self.push();
        self.block_stmts(b);
        self.pop();
    }

    fn block_stmts(&mut self, b: &'a Block) {
        let top = self.top_of_body;
        for s in &b.stmts {
            self.top_of_body = top;
            if top && matches!(&s.kind, StmtKind::Expr(e) if is_task(e)) {
                self.task(match &s.kind {
                    StmtKind::Expr(e) => e,
                    _ => unreachable!(),
                });
                continue;
            }
            self.stmt(s);
        }
        self.top_of_body = top;
    }

    fn stmt(&mut self, s: &'a Stmt) {
        match &s.kind {
            StmtKind::Binding(b) => self.binding(b),
            StmtKind::Let { pattern, mutable, value, otherwise } => {
                match (pattern, &value.kind) {
                    // A local function: declared first, so it can call itself.
                    (Pattern::Name(n), ExprKind::Lambda(l)) if !mutable => {
                        self.declare(n, BindKind::Let);
                        self.lambda(l, false);
                        return;
                    }
                    _ => self.expr(value),
                }
                match (pattern, otherwise) {
                    (Pattern::Tag { name, .. }, None) => self.error(
                        "refutable",
                        format!(
                            "the pattern must cover every case; `{}` is only one of them. Add `else {{ … }}` for the others, or use `match`",
                            name.name
                        ),
                        pattern.span(),
                    ),
                    (_, Some(b)) => {
                        // The `else` runs when the pattern doesn't match, so its
                        // names aren't bound there, and it must leave.
                        self.block(b);
                        if !leaves(b) {
                            self.error(
                                "let-else",
                                "the `else` of `let … else` must leave: end it with `return`, `break`, `continue` or `crash`",
                                b.span,
                            );
                        }
                    }
                    _ => {}
                }
                let kind = if *mutable { BindKind::Var { owner: self.lambda_id } } else { BindKind::Let };
                self.pattern_bind(pattern, kind);
            }
            StmtKind::State { name, value, .. } => {
                if !self.client {
                    self.error("state-outside", "`state` only works in a component or a stateful function", name.span);
                }
                self.expr(value);
                if self.lookup(&name.name).map(|b| b.kind) != Some(BindKind::State) {
                    self.declare(name, BindKind::State);
                }
            }
            StmtKind::Style => {}
            StmtKind::For { pattern, iterable, body } => {
                self.top_of_body = false;
                self.expr(iterable);
                self.push();
                self.pattern_bind(pattern, BindKind::Let);
                self.block_stmts(body);
                self.pop();
            }
            StmtKind::While { condition, body } => {
                self.top_of_body = false;
                self.expr(condition);
                self.block(body);
            }
            StmtKind::Break | StmtKind::Continue => {}
            StmtKind::Return(e) => {
                if let Some(e) = e {
                    self.expr(e);
                }
            }
            StmtKind::With { handler, body } => {
                self.expr(handler);
                self.top_of_body = false;
                self.block(body);
            }
            StmtKind::Defer(b) => {
                self.top_of_body = false;
                self.block(b);
            }
            StmtKind::Expr(e) => {
                self.must_use(e);
                self.expr_or_assign(e);
            }
        }
    }

    /// `name = value` in a block: an assignment.
    fn binding(&mut self, b: &'a Binding) {
        let Pattern::Name(n) = &b.left else { return };
        self.expr(&b.value);
        let found = self.lookup(&n.name);
        match found.map(|x| x.kind) {
            Some(BindKind::State) | Some(BindKind::Unknown) => self.assign_state(n),
            Some(BindKind::Var { owner }) => self.assign_var(n, owner, found.and_then(|f| f.span)),
            Some(BindKind::Let) | Some(BindKind::Param) | Some(BindKind::Global) => {
                let mut d = Diagnostic::error(
                    "immutable",
                    format!(
                        "`{}` isn't mutable: write `let {} = …` for a new binding, or declare it with `let mut`",
                        n.name, n.name
                    ),
                    n.span,
                );
                if let Some(s) = found.and_then(|f| f.span) {
                    d = d.with_label(s, "bound here, without `mut`");
                }
                self.diags.push((self.file, d));
            }
            None if self.strict => self.error(
                "unknown-name",
                format!("unknown name `{}`: write `let {} = …` to bind it", n.name, n.name),
                n.span,
            ),
            // A snippet assigning a name it doesn't declare: most likely a
            // component's `state` defined elsewhere.
            None => {
                self.assign_state(n);
                if let Some(sc) = self.scopes.last_mut() {
                    sc.insert(n.name.clone(), Binder { kind: BindKind::Unknown, span: None });
                }
            }
        }
    }

    fn assign_state(&mut self, n: &Ident) {
        self.effect(Effect::Assign(n.name.clone()), n.span);
    }

    fn assign_var(&mut self, n: &Ident, owner: usize, declared: Option<Span>) {
        if owner != self.lambda_id {
            let mut d = Diagnostic::error(
                "var-escape",
                format!("only the function that declared `{}` can assign it", n.name),
                n.span,
            );
            if let Some(s) = declared {
                d = d.with_label(s, "declared here, in an enclosing function");
            }
            self.diags.push((self.file, d));
        }
    }

    fn pattern_bind(&mut self, p: &Pattern, kind: BindKind) {
        if let Pattern::Or(alts, span) = p {
            // Every alternative must bind the same names.
            let names = |q: &Pattern| {
                let mut v: Vec<String> = q.names().iter().map(|i| i.name.clone()).collect();
                v.sort();
                v
            };
            if let Some(first) = alts.first() {
                let want = names(first);
                if alts.iter().any(|a| names(a) != want) {
                    self.error("or-pattern", "each side of `|` must bind the same names", *span);
                }
            }
        }
        for n in p.names() {
            self.declare(n, kind);
        }
    }

    /// A `Result` can't be ignored: sending a command that can fail, as a
    /// statement, without handling what it returns.
    fn must_use(&mut self, e: &Expr) {
        let call = match &e.kind {
            ExprKind::Await(inner) => inner.as_ref(),
            _ => e,
        };
        if let ExprKind::Call { function, .. } = &call.kind {
            if let ExprKind::Name(n) = &function.kind {
                if let Some(Def { kind: DefKind::Command { lambda }, .. }) = self.global(n) {
                    if let Some(Type::Name { name, args }) = &lambda.result {
                        if name.name == "Result" {
                            let reasons = args.get(1).map(crate::program::type_text).unwrap_or_default();
                            self.error(
                                "unused-result",
                                format!("{n} can fail with {reasons}; await it and handle the Result"),
                                e.span,
                            );
                        }
                    }
                }
            }
        }
    }

    // ── Expressions ───────────────────────────────────────────

    fn expr_or_assign(&mut self, e: &'a Expr) {
        self.expr(e);
    }

    fn expr(&mut self, e: &'a Expr) {
        match &e.kind {
            ExprKind::Name(n) => self.resolve_value(n, e.span),
            ExprKind::Tag(_) | ExprKind::Number { .. } | ExprKind::Accessor(_) | ExprKind::Unit | ExprKind::Error => {}
            ExprKind::Text(parts) => {
                for p in parts {
                    if let TextPart::Interpolation(x) = p {
                        self.expr(x);
                    }
                }
            }
            ExprKind::List(items) => items.iter().for_each(|i| self.expr(i)),
            ExprKind::TypedRecord { ty, fields } if self.g.types.contains_key(&ty.name) => {
                // Building a record runs the defaults of the fields left out.
                self.units[self.unit].calls.push((format!("{} defaults", ty.name), e.span));
                for f in fields {
                    match f {
                        Field::Named { value, .. } | Field::Spread(value) => self.expr(value),
                        Field::Shorthand(i) => self.resolve_value(&i.name, i.span),
                    }
                }
            }
            ExprKind::Record(fields) | ExprKind::TypedRecord { fields, .. } => {
                for f in fields {
                    match f {
                        Field::Named { value, .. } | Field::Spread(value) => self.expr(value),
                        Field::Shorthand(i) => self.resolve_value(&i.name, i.span),
                    }
                }
            }
            ExprKind::Block(b) => {
                let saved = self.top_of_body;
                self.top_of_body = false;
                self.block(b);
                self.top_of_body = saved;
            }
            ExprKind::Lambda(l) => self.deferred_lambda(l, e.span),
            ExprKind::If { pattern, condition, then, otherwise } => {
                let saved = self.top_of_body;
                self.top_of_body = false;
                self.expr(condition);
                self.push();
                if let Some(p) = pattern {
                    self.pattern_bind(p, BindKind::Let);
                }
                self.block(then);
                self.pop();
                if let Some(o) = otherwise {
                    self.expr(o);
                }
                self.top_of_body = saved;
            }
            ExprKind::Match { value, arms } => {
                let saved = self.top_of_body;
                self.top_of_body = false;
                self.expr(value);
                for a in arms {
                    self.push();
                    self.pattern_bind(&a.pattern, BindKind::Let);
                    if let Some(g) = &a.guard {
                        self.expr(g);
                    }
                    self.expr(&a.value);
                    self.pop();
                }
                self.top_of_body = saved;
            }
            ExprKind::Call { function, args } => self.call(e, function, args),
            ExprKind::Element { head, children } => {
                if is_task(e) {
                    self.error(
                        "task-placement",
                        "a `task` goes at the top of a component or stateful function, not inside other code",
                        e.span,
                    );
                }
                self.element_head(head);
                let saved = self.top_of_body;
                self.top_of_body = false;
                self.block(children);
                self.top_of_body = saved;
            }
            ExprKind::Field { value, .. } => self.expr(value),
            ExprKind::Await(inner) => {
                self.effect(Effect::Await, e.span);
                self.expr(inner);
            }
            ExprKind::Try(inner) | ExprKind::Crash(inner) => self.expr(inner),
            ExprKind::Unary { operand, .. } => self.expr(operand),
            ExprKind::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Assign { name, value } => {
                self.expr(value);
                match self.lookup(&name.name).map(|b| b.kind) {
                    Some(BindKind::State) | Some(BindKind::Unknown) => self.assign_state(name),
                    Some(BindKind::Var { owner }) => {
                        let declared = self.lookup(&name.name).and_then(|b| b.span);
                        self.assign_var(name, owner, declared)
                    }
                    None if !self.strict => {}
                    _ => self.error(
                        "assign",
                        format!("`{}` isn't a `state` or `var`, so it can't be assigned", name.name),
                        name.span,
                    ),
                }
            }
        }
    }

    /// The name part of `name(attrs) { children }`.
    fn element_head(&mut self, head: &'a Expr) {
        match &head.kind {
            ExprKind::Name(n) => self.resolve_element(n, head.span),
            ExprKind::Call { function, args } => {
                match &function.kind {
                    ExprKind::Name(n) => self.resolve_element(n, function.span),
                    _ => self.expr(function),
                }
                for a in args {
                    self.expr(&a.value);
                }
            }
            _ => self.expr(head),
        }
    }

    fn resolve_element(&mut self, n: &str, span: Span) {
        if builtins::ELEMENTS.contains(&n) || n == "task" {
            return;
        }
        self.resolve_value(n, span);
    }

    fn call(&mut self, e: &'a Expr, function: &'a Expr, args: &'a [Arg]) {
        match &function.kind {
            ExprKind::Name(n) => {
                if let Some(eff) = builtins::function_effect(n) {
                    if self.lookup(n).is_none() {
                        self.effect(eff, e.span);
                    }
                }
                if builtins::ELEMENTS.contains(&n.as_str()) && self.lookup(n).is_none() {
                    // An element without children: `input(bind: draft)`.
                } else {
                    self.resolve_value(n, function.span);
                }
                if let Some(def) = self.global(n) {
                    match &def.kind {
                        DefKind::Query { .. } => {
                            if self.client {
                                self.effect(Effect::CallQuery(n.clone()), e.span);
                            } else {
                                self.units[self.unit].calls.push((n.clone(), e.span));
                            }
                        }
                        DefKind::Command { .. } => {
                            if self.client {
                                self.effect(Effect::Send(n.clone()), e.span);
                            } else {
                                self.units[self.unit].calls.push((n.clone(), e.span));
                            }
                        }
                        DefKind::Function { stateful } => {
                            if *stateful && !self.top_of_body {
                                self.error(
                                    "stateful-call",
                                    format!(
                                        "`{n}` is a stateful function: call it at the top of a component or stateful function, not inside an `if`, a loop or a lambda"
                                    ),
                                    e.span,
                                );
                            }
                            self.units[self.unit].calls.push((n.clone(), e.span));
                        }
                        DefKind::Extern => self.effect(Effect::Extern(n.clone()), e.span),
                        _ => {}
                    }
                }
                self.args(args, false);
            }
            ExprKind::Field { value, field } => self.method_call(e, value, field, args),
            _ => {
                self.expr(function);
                self.args(args, false);
            }
        }
    }

    fn method_call(&mut self, e: &'a Expr, receiver: &'a Expr, method: &Ident, args: &'a [Arg]) {
        let m = method.name.as_str();
        if let ExprKind::Name(r) = &receiver.kind {
            if builtins::MODULES.contains(&r.as_str()) && self.lookup(r).is_none() {
                self.effect(builtins::module_effect(r, m), e.span);
                self.args(args, false);
                return;
            }
            if let Some(Def { kind: DefKind::Table(scope), .. }) = self.global(r) {
                let scope = *scope;
                let read = builtins::TABLE_READS.contains(&m);
                let write = builtins::TABLE_WRITES.contains(&m);
                if !read && !write {
                    self.error("table-method", format!("tables have no method `{m}`"), method.span);
                }
                // `update` reads the row it changes, and so do the `_where`
                // methods: they're writes here.
                if write {
                    self.effect(Effect::Write(table_label(r, scope)), e.span);
                } else {
                    self.effect(Effect::Read(table_label(r, scope)), e.span);
                }
                if read && !write {
                    for a in args {
                        if let ExprKind::Lambda(l) = &a.value.kind {
                            let mut seen = HashSet::new();
                            self.query_fragment(&l.body, r, m, &mut seen);
                        }
                    }
                }
                self.args(args, true);
                return;
            }
        }
        self.expr(receiver);
        if let Some(eff) = builtins::handle_method_effect(m) {
            if !builtins::IMMEDIATE_METHODS.contains(&m) {
                self.effect(eff, e.span);
            }
        }
        self.args(args, builtins::IMMEDIATE_METHODS.contains(&m));
    }

    /// Arguments: lambdas run right away (a list method's function) are part
    /// of this unit; others (handlers, stored functions) are deferred.
    fn args(&mut self, args: &'a [Arg], immediate: bool) {
        for a in args {
            match &a.value.kind {
                ExprKind::Lambda(l) if immediate => {
                    let saved = self.top_of_body;
                    self.top_of_body = false;
                    self.lambda(l, false);
                    self.top_of_body = saved;
                }
                _ => {
                    let saved = self.top_of_body;
                    self.top_of_body = false;
                    self.expr(&a.value);
                    self.top_of_body = saved;
                }
            }
        }
    }

    // ── The query fragment ────────────────────────────────────

    /// Checks that a function given to a table method can run as a query
    /// (reference: Data, "The query fragment").
    fn query_fragment(&mut self, e: &Expr, table: &str, method: &str, seen: &mut HashSet<String>) {
        let mut problem: Option<(String, Span)> = None;
        fragment_problem(e, self.g, self.sources, seen, &mut problem);
        if let Some((why, span)) = problem {
            self.error(
                "query-fragment",
                format!(
                    "this can't run as a query: {why}. To {method} in memory, fetch the rows first: {table}.rows().{method}(…)"
                ),
                span,
            );
        }
    }
}

/// Whether a block always leaves: its last statement returns, breaks,
/// continues or crashes.
fn leaves(b: &Block) -> bool {
    match b.stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Return(_)) | Some(StmtKind::Break) | Some(StmtKind::Continue) => true,
        Some(StmtKind::Expr(Expr { kind: ExprKind::Crash(_), .. })) => true,
        _ => false,
    }
}

fn table_label(name: &str, scope: TableScope) -> String {
    match scope {
        TableScope::Shared => name.to_string(),
        TableScope::Device => format!("{name} (device)"),
        TableScope::Memory => format!("{name} (memory)"),
    }
}

fn fragment_problem(e: &Expr, g: &Globals, sources: &[Source], seen: &mut HashSet<String>, out: &mut Option<(String, Span)>) {
    if out.is_some() {
        return;
    }
    match &e.kind {
        ExprKind::Block(b) => {
            for s in &b.stmts {
                match &s.kind {
                    StmtKind::Let { mutable: true, .. } => return set_problem(out, "it declares a `let mut`".into(), s.span),
                    StmtKind::Let { value, .. } => fragment_problem(value, g, sources, seen, out),
                    StmtKind::For { .. } => return set_problem(out, "it uses a `for` loop".into(), s.span),
                    StmtKind::While { .. } => return set_problem(out, "it uses a `while` loop".into(), s.span),
                    StmtKind::Binding(bd) => fragment_problem(&bd.value, g, sources, seen, out),
                    StmtKind::Expr(x) => fragment_problem(x, g, sources, seen, out),
                    StmtKind::Return(Some(x)) => fragment_problem(x, g, sources, seen, out),
                    _ => return set_problem(out, "it has a statement with no query form".into(), s.span),
                }
            }
        }
        ExprKind::Await(_) => set_problem(out, "it waits with `.await`".into(), e.span),
        ExprKind::Assign { .. } => set_problem(out, "it assigns a variable".into(), e.span),
        ExprKind::Crash(_) => set_problem(out, "it can `crash`".into(), e.span),
        ExprKind::Call { function, args } => {
            match &function.kind {
                ExprKind::Name(n) => {
                    if let Some(Def { kind: DefKind::Function { .. }, file, .. }) = g.values.get(n) {
                        if !seen.insert(n.clone()) {
                            return set_problem(out, format!("`{n}` is recursive"), e.span);
                        }
                        if let Some(body) = function_body(&sources[*file], n) {
                            let mut inner = None;
                            fragment_problem(body, g, sources, seen, &mut inner);
                            if let Some((why, _)) = inner {
                                return set_problem(out, format!("`{n}` {}", why.trim_start_matches("it ")), e.span);
                            }
                        }
                    } else if builtins::function_effect(n).is_some() || g.values.get(n).is_some() {
                        // Built-ins such as `now()` don't depend on the row and
                        // are computed before the query runs.
                    }
                }
                ExprKind::Field { value, field } => {
                    let is_table = matches!(&value.kind, ExprKind::Name(t) if matches!(g.values.get(t), Some(Def { kind: DefKind::Table(_), .. })));
                    if !is_table && !builtins::QUERY_METHODS.contains(&field.name.as_str()) {
                        return set_problem(out, format!("`{}` has no query form", field.name), field.span);
                    }
                    fragment_problem(value, g, sources, seen, out);
                }
                _ => fragment_problem(function, g, sources, seen, out),
            }
            for a in args {
                if let ExprKind::Lambda(l) = &a.value.kind {
                    fragment_problem(&l.body, g, sources, seen, out);
                } else {
                    fragment_problem(&a.value, g, sources, seen, out);
                }
            }
        }
        ExprKind::If { condition, then, otherwise, .. } => {
            fragment_problem(condition, g, sources, seen, out);
            fragment_problem(&Expr { kind: ExprKind::Block(then.clone()), span: then.span }, g, sources, seen, out);
            if let Some(o) = otherwise {
                fragment_problem(o, g, sources, seen, out);
            }
        }
        ExprKind::Match { value, arms } => {
            fragment_problem(value, g, sources, seen, out);
            for a in arms {
                fragment_problem(&a.value, g, sources, seen, out);
            }
        }
        ExprKind::Field { value, .. } | ExprKind::Try(value) => fragment_problem(value, g, sources, seen, out),
        ExprKind::Unary { operand, .. } => fragment_problem(operand, g, sources, seen, out),
        ExprKind::Binary { left, right, .. } => {
            fragment_problem(left, g, sources, seen, out);
            fragment_problem(right, g, sources, seen, out);
        }
        ExprKind::Record(fields) | ExprKind::TypedRecord { fields, .. } => {
            for f in fields {
                if let Field::Named { value, .. } | Field::Spread(value) = f {
                    fragment_problem(value, g, sources, seen, out);
                }
            }
        }
        ExprKind::List(items) => items.iter().for_each(|i| fragment_problem(i, g, sources, seen, out)),
        _ => {}
    }
}

fn set_problem(out: &mut Option<(String, Span)>, why: String, span: Span) {
    if out.is_none() {
        *out = Some((why, span));
    }
}

fn function_body<'s>(src: &'s Source, name: &str) -> Option<&'s Expr> {
    for item in &src.file.items {
        if let ItemKind::Binding(b) = &item.kind {
            if matches!(&b.left, Pattern::Name(n) if n.name == name) {
                if let ExprKind::Lambda(l) = &b.value.kind {
                    return Some(&l.body);
                }
            }
        }
    }
    None
}
