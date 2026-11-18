//! weft's syntax: the tree-sitter parser (generated from
//! `tree-sitter-weft/grammar.js`), syntax errors, and the typed AST.

pub mod ast;
pub mod diag;
pub mod fmt;
mod lower;

pub use diag::{Diagnostic, Severity};

use ast::{File, Span};
use tree_sitter::{Node, Parser};
use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_weft() -> *const ();
}

/// The tree-sitter language for weft.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_weft) };

pub struct Parsed {
    pub file: File,
    pub diagnostics: Vec<Diagnostic>,
    /// The concrete tree's S-expression, for debugging.
    pub sexp: String,
}

/// Parses one weft source file.
pub fn parse(src: &str) -> Parsed {
    let mut parser = Parser::new();
    parser.set_language(&LANGUAGE.into()).expect("the weft grammar's ABI matches the tree-sitter crate");
    let tree = parser.parse(src, None).expect("parsing never times out");
    let root = tree.root_node();
    let mut diagnostics = Vec::new();
    syntax_errors(root, src, &mut diagnostics);
    let mut lowerer = lower::Lowerer::new(src);
    let file = lowerer.file(root);
    diagnostics.extend(lowerer.diags);
    Parsed { file, diagnostics, sexp: root.to_sexp() }
}

/// Parses a snippet as the statements of a block.
pub fn parse_wrapped(src: &str) -> (Parsed, SnippetWrap) {
    let indented: String = src.lines().map(|l| format!("  {l}\n")).collect();
    let wrapped = format!("__snippet = || {{\n{indented}}}\n");
    (parse(&wrapped), SnippetWrap { line_offset: 1, col_offset: 2 })
}

/// Parses a documentation snippet: a whole file if it is one, otherwise the
/// statements of a block. Returns the parse and how many bytes of wrapper were
/// added before the snippet (to map spans back).
pub fn parse_snippet(src: &str) -> (Parsed, Option<SnippetWrap>) {
    let whole = parse(src);
    if whole.diagnostics.iter().all(|d| d.code != "syntax") {
        return (whole, None);
    }
    let indented: String = src.lines().map(|l| format!("  {l}\n")).collect();
    let wrapped = format!("__snippet = || {{\n{indented}}}\n");
    let parsed = parse(&wrapped);
    let wrapped_errors = parsed.diagnostics.iter().filter(|d| d.code == "syntax").count();
    let whole_errors = whole.diagnostics.iter().filter(|d| d.code == "syntax").count();
    if wrapped_errors < whole_errors {
        (parsed, Some(SnippetWrap { line_offset: 1, col_offset: 2 }))
    } else {
        (whole, None)
    }
}

/// How a snippet was wrapped, so diagnostics can point at the original lines.
#[derive(Clone, Copy, Debug)]
pub struct SnippetWrap {
    pub line_offset: usize,
    pub col_offset: usize,
}

impl SnippetWrap {
    pub fn unwrap_span(&self, s: Span) -> Span {
        Span {
            line: s.line.saturating_sub(self.line_offset),
            col: s.col.saturating_sub(self.col_offset),
            ..s
        }
    }
}

fn syntax_errors(n: Node, src: &str, out: &mut Vec<Diagnostic>) {
    if n.is_missing() {
        out.push(Diagnostic::error("syntax", format!("expected `{}` here", n.kind()), node_span(n)));
        return;
    }
    if n.is_error() {
        let text = &src[n.byte_range()];
        let first_line = text.lines().next().unwrap_or("").trim();
        let shown: String = first_line.chars().take(40).collect();
        out.push(Diagnostic::error("syntax", format!("can't parse `{shown}`"), node_span(n)));
        return;
    }
    if !n.has_error() {
        return;
    }
    let mut c = n.walk();
    for child in n.children(&mut c) {
        syntax_errors(child, src, out);
    }
}

fn node_span(n: Node) -> Span {
    let p = n.start_position();
    Span { start: n.start_byte(), end: n.end_byte(), line: p.row, col: p.column }
}
