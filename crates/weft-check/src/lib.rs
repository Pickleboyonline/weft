//! The weft checker: name resolution, effect inference, type inference and
//! the language's rules. It produces diagnostics only; nothing is compiled.

pub mod builtins;
pub mod effects;
pub mod explain;
pub mod program;
pub mod rules;
pub mod types;
pub mod walk;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use weft_syntax::ast::Span;
use weft_syntax::Diagnostic;

pub use program::Source;
use program::{type_text, DefKind, Globals};
use rules::EffectSet;
use walk::{Unit, UnitKind, Walker};

pub struct Report {
    /// Every diagnostic, with the index of its source.
    pub diagnostics: Vec<(usize, Diagnostic)>,
    pub units: Vec<Unit>,
    pub totals: Vec<EffectSet>,
    pub globals: Globals,
}

/// Parses a file as a whole project file.
pub fn source(path: impl Into<String>, text: impl Into<String>) -> Source {
    let text = text.into();
    let parsed = weft_syntax::parse(&text);
    Source { path: path.into(), file: parsed.file, diags: parsed.diagnostics, wrap: None, text }
}

/// Parses a documentation snippet, which may be statements rather than a file.
/// A snippet is read as a file when it parses as one and every binding in it
/// defines a function, a remote function or a configuration value; otherwise
/// (`item = Item { … }`, `cleaned = title.trim()`) its lines are statements.
pub fn snippet(path: impl Into<String>, text: impl Into<String>) -> Source {
    let text = text.into();
    let whole = weft_syntax::parse(&text);
    let parses = whole.diagnostics.iter().all(|d| d.code != "syntax");
    let definitions = whole.file.items.iter().all(|i| match &i.kind {
        weft_syntax::ast::ItemKind::Binding(b) => is_definition(&b.value),
        _ => true,
    });
    if parses && (definitions || whole.file.items.iter().any(|i| !matches!(i.kind, weft_syntax::ast::ItemKind::Binding(_)))) {
        return Source { path: path.into(), file: whole.file, diags: whole.diagnostics, wrap: None, text };
    }
    let (wrapped, wrap) = weft_syntax::parse_wrapped(&text);
    let wrapped_ok = wrapped.diagnostics.iter().all(|d| d.code != "syntax");
    if wrapped_ok || !parses {
        let (p, w) = if wrapped_ok { (wrapped, Some(wrap)) } else { weft_syntax::parse_snippet(&text) };
        return Source { path: path.into(), file: p.file, diags: p.diagnostics, wrap: w, text };
    }
    Source { path: path.into(), file: whole.file, diags: whole.diagnostics, wrap: None, text }
}

fn is_definition(value: &weft_syntax::ast::Expr) -> bool {
    use weft_syntax::ast::ExprKind;
    match &value.kind {
        ExprKind::Lambda(_) => true,
        ExprKind::Call { function, .. } => matches!(function.kind, ExprKind::Tag(_)),
        ExprKind::TypedRecord { .. } => true,
        _ => false,
    }
}

/// Every `.weft` file under `dir`, sorted.
pub fn load_project(dir: &Path) -> std::io::Result<Vec<Source>> {
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    files
        .into_iter()
        .map(|p| Ok(source(p.display().to_string(), std::fs::read_to_string(&p)?)))
        .collect()
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        if p.is_dir() {
            collect(&p, out)?;
        } else if p.extension().is_some_and(|e| e == "weft") {
            out.push(p);
        }
    }
    Ok(())
}

/// Checks a set of sources. `strict` is for whole projects: unknown names and
/// duplicate definitions are errors. Snippets from the docs use `strict:
/// false`, since they refer to things defined elsewhere.
pub fn check(sources: &[Source], strict: bool) -> Report {
    let mut diagnostics: Vec<(usize, Diagnostic)> = Vec::new();
    for (i, s) in sources.iter().enumerate() {
        diagnostics.extend(s.diags.iter().cloned().map(|d| (i, d)));
    }
    let globals = Globals::collect(sources, strict, &mut diagnostics);
    let mut walker = Walker::new(&globals, sources, strict);
    walker.walk_program();
    diagnostics.extend(walker.diags);
    let units = walker.units;
    let totals = rules::propagate(&units);
    rules::check(&units, &totals, &mut diagnostics);
    diagnostics.extend(types::check(sources, &globals, strict));
    diagnostics.sort_by_key(|(f, d)| (*f, d.span.line, d.span.col));
    diagnostics.dedup_by(|a, b| a.0 == b.0 && a.1.span == b.1.span && a.1.message == b.1.message);
    Report { diagnostics, units, totals, globals }
}

impl Report {
    /// `weft explain effects`: every function and component, and what it does.
    pub fn explain_effects(&self) -> String {
        let mut rows: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (u, t) in self.units.iter().zip(&self.totals) {
            let key = match u.kind {
                UnitKind::Function | UnitKind::Stateful | UnitKind::Query | UnitKind::Command | UnitKind::Check => {
                    u.name.clone()
                }
                UnitKind::View | UnitKind::Handler | UnitKind::Task => match &u.owner {
                    Some(c) => c.clone(),
                    None => continue,
                },
                _ => continue,
            };
            let entry = rows.entry(key).or_default();
            for e in t.keys() {
                let s = e.to_string();
                if !entry.contains(&s) {
                    entry.push(s);
                }
            }
        }
        let width = rows.keys().map(|k| k.len()).max().unwrap_or(0) + 2;
        rows.iter()
            .map(|(k, v)| {
                let v = if v.is_empty() { "pure".to_string() } else { v.join("; ") };
                format!("{k:width$}{v}\n")
            })
            .collect()
    }

    /// `weft explain api`: every query and command, with its types.
    pub fn explain_api(&self) -> String {
        let mut rows: Vec<(String, String)> = Vec::new();
        for d in self.globals.values.values() {
            let (what, lambda) = match &d.kind {
                DefKind::Query { lambda } => ("Query  ", lambda),
                DefKind::Command { lambda } => ("Command", lambda),
                _ => continue,
            };
            let params = lambda
                .params
                .iter()
                .map(|p| p.ty.as_ref().map(type_text).unwrap_or_else(|| "?".into()))
                .collect::<Vec<_>>()
                .join(", ");
            let result = lambda.result.as_ref().map(type_text).unwrap_or_else(|| "?".into());
            let params = if params.is_empty() { "()".to_string() } else { params };
            rows.push((d.name.clone(), format!("{what} {params} -> {result}")));
        }
        rows.sort();
        let width = rows.iter().map(|r| r.0.len()).max().unwrap_or(0) + 2;
        rows.iter().map(|(n, s)| format!("{n:width$}{s}\n")).collect()
    }
}

/// Maps a diagnostic's span back to the snippet the user wrote.
pub fn original_span(src: &Source, span: Span) -> Span {
    match &src.wrap {
        Some(w) => w.unwrap_span(span),
        None => span,
    }
}
