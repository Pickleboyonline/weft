//! The whole program: its source files and the top-level names they define.

use std::collections::HashMap;
use weft_syntax::ast::*;
use weft_syntax::{Diagnostic, SnippetWrap};

pub struct Source {
    pub path: String,
    pub text: String,
    pub file: File,
    pub diags: Vec<Diagnostic>,
    pub wrap: Option<SnippetWrap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableScope {
    Shared,
    Device,
    Memory,
}

#[derive(Clone, Debug)]
pub enum DefKind {
    Function { stateful: bool },
    Constant,
    Query { lambda: Lambda },
    Command { lambda: Lambda },
    Check,
    Table(TableScope),
    /// `Job { … }`, `Deployment { … }` and other configuration values.
    Config(String),
    Extern,
    Record,
    Component,
    Effect,
    Handler,
    Alias,
}

#[derive(Clone, Debug)]
pub struct Def {
    pub name: String,
    pub kind: DefKind,
    pub span: Span,
    pub file: usize,
}

#[derive(Default)]
pub struct Globals {
    pub values: HashMap<String, Def>,
    pub types: HashMap<String, Def>,
    pub signatures: HashMap<String, Signature>,
}

impl Globals {
    pub fn collect(sources: &[Source], strict: bool, diags: &mut Vec<(usize, Diagnostic)>) -> Globals {
        let mut g = Globals::default();
        for (fi, src) in sources.iter().enumerate() {
            for item in &src.file.items {
                let (name, kind, is_type) = match &item.kind {
                    ItemKind::Binding(b) => match &b.left {
                        Pattern::Name(n) => (n.clone(), classify(&b.value), false),
                        _ => continue,
                    },
                    ItemKind::Record(r) => (r.name.clone(), DefKind::Record, true),
                    ItemKind::Component(c) => (c.name.clone(), DefKind::Component, false),
                    ItemKind::Effect(e) => (e.name.clone(), DefKind::Effect, true),
                    ItemKind::Handler(h) => (h.name.clone(), DefKind::Handler, false),
                    ItemKind::Alias(a) => (a.name.clone(), DefKind::Alias, true),
                    ItemKind::Signature(s) => {
                        g.signatures.insert(s.name.name.clone(), s.clone());
                        continue;
                    }
                    ItemKind::Expect(_) => continue,
                };
                let map = if is_type { &mut g.types } else { &mut g.values };
                if let Some(prev) = map.get(&name.name) {
                    // In snippet mode the docs' prelude (the example project)
                    // may define the same names; only report duplicates
                    // within one file.
                    if strict || prev.file == fi {
                        diags.push((
                            fi,
                            Diagnostic::error(
                                "duplicate",
                                format!(
                                    "`{}` is defined twice; the first is in {} line {}",
                                    name.name,
                                    sources[prev.file].path,
                                    prev.span.line + 1
                                ),
                                name.span,
                            ),
                        ));
                    }
                    continue;
                }
                map.insert(name.name.clone(), Def { name: name.name.clone(), kind, span: name.span, file: fi });
            }
        }
        g
    }
}

/// What a top-level binding's value makes it.
fn classify(value: &Expr) -> DefKind {
    match &value.kind {
        ExprKind::Lambda(l) => DefKind::Function { stateful: is_stateful(&l.body) },
        ExprKind::Call { function, args } => match &function.kind {
            ExprKind::Tag(t) => {
                let lambda = args.first().and_then(|a| match &a.value.kind {
                    ExprKind::Lambda(l) => Some(l.clone()),
                    _ => None,
                });
                match (t.as_str(), lambda) {
                    ("Query", Some(lambda)) => DefKind::Query { lambda },
                    ("Command", Some(lambda)) => DefKind::Command { lambda },
                    ("Check", _) => DefKind::Check,
                    ("Table", _) => DefKind::Table(table_scope(args)),
                    (other, _) => DefKind::Config(other.to_string()),
                }
            }
            _ => DefKind::Constant,
        },
        ExprKind::TypedRecord { ty, .. } if ty.name == "Extern" => DefKind::Extern,
        ExprKind::TypedRecord { ty, .. } if crate::builtins::TYPES.contains(&ty.name.as_str()) => {
            DefKind::Config(ty.name.clone())
        }
        _ => DefKind::Constant,
    }
}

fn table_scope(args: &[Arg]) -> TableScope {
    for a in args {
        if a.name.as_ref().is_some_and(|n| n.name == "scope") {
            if let ExprKind::Tag(t) = &a.value.kind {
                return match t.as_str() {
                    "Device" => TableScope::Device,
                    "Memory" => TableScope::Memory,
                    _ => TableScope::Shared,
                };
            }
        }
    }
    TableScope::Shared
}

/// A function is stateful when its body declares `state` or a `task`.
pub fn is_stateful(body: &Expr) -> bool {
    match &body.kind {
        ExprKind::Block(b) => b.stmts.iter().any(|s| match &s.kind {
            StmtKind::State { .. } => true,
            StmtKind::Expr(e) => is_task(e),
            _ => false,
        }),
        _ => false,
    }
}

/// `task { … }` or `task(key: k) { … }`.
pub fn is_task(e: &Expr) -> bool {
    if let ExprKind::Element { head, .. } = &e.kind {
        return match &head.kind {
            ExprKind::Name(n) => n == "task",
            ExprKind::Call { function, .. } => matches!(&function.kind, ExprKind::Name(n) if n == "task"),
            _ => false,
        };
    }
    false
}

/// Renders a type the way the reference writes it.
pub fn type_text(t: &Type) -> String {
    match t {
        Type::Name { name, args } if args.is_empty() => name.name.clone(),
        Type::Name { name, args } => {
            format!("{}({})", name.name, args.iter().map(type_text).collect::<Vec<_>>().join(", "))
        }
        Type::Var(v) => v.name.clone(),
        Type::Fn { params, result, .. } => format!(
            "{} -> {}",
            params.iter().map(type_text).collect::<Vec<_>>().join(", "),
            type_text(result)
        ),
        Type::Record { fields, .. } if fields.is_empty() => "{}".into(),
        Type::Record { fields, .. } => format!(
            "{{ {} }}",
            fields.iter().map(|(n, t)| format!("{}: {}", n.name, type_text(t))).collect::<Vec<_>>().join(", ")
        ),
        Type::Tags { tags, open, .. } => {
            let mut parts: Vec<String> = tags
                .iter()
                .map(|(n, a)| {
                    if a.is_empty() {
                        n.name.clone()
                    } else {
                        format!("{}({})", n.name, a.iter().map(type_text).collect::<Vec<_>>().join(", "))
                    }
                })
                .collect();
            if *open {
                parts.push("..".into());
            }
            format!("[{}]", parts.join(", "))
        }
        Type::Unit(_) => "()".into(),
        Type::Error(_) => "?".into(),
    }
}
