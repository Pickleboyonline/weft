//! Diagnostics shared by the parser and the checker.

use crate::ast::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    /// A stable short code, e.g. `query-effect`. `weft explain <code>` says more.
    pub code: &'static str,
    /// Written as a fix instruction where possible.
    pub message: String,
    pub span: Span,
    /// Other places that matter: "bound here", "declared here".
    pub labels: Vec<(Span, String)>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Diagnostic { severity: Severity::Error, code, message: message.into(), span, labels: vec![], notes: vec![] }
    }

    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Diagnostic { severity: Severity::Warning, code, message: message.into(), span, labels: vec![], notes: vec![] }
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push((span, message.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Renders in the style of rustc: the message, the source line with a
    /// caret, secondary labels, notes, and how to learn more.
    pub fn render(&self, path: &str, src: &str) -> String {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let gutter = (self.labels.iter().map(|(s, _)| s.line).chain([self.span.line]).max().unwrap_or(0) + 1)
            .to_string()
            .len();
        let pad = " ".repeat(gutter);
        let mut out = format!(
            "{sev}[{}]: {}\n{pad}--> {path}:{}:{}\n",
            self.code,
            self.message,
            self.span.line + 1,
            self.span.col + 1
        );
        out += &snippet(src, self.span, '^', "", gutter);
        for (span, msg) in &self.labels {
            out += &snippet(src, *span, '-', msg, gutter);
        }
        for n in &self.notes {
            out += &format!("{pad} = note: {n}\n");
        }
        out += &format!("{pad} = help: `weft explain {}` says more\n", self.code);
        out
    }
}

fn snippet(src: &str, span: Span, mark: char, label: &str, gutter: usize) -> String {
    let line_text = src.lines().nth(span.line).unwrap_or("");
    let avail = line_text.len().saturating_sub(span.col).max(1);
    let width = span.end.saturating_sub(span.start).clamp(1, avail);
    let label = if label.is_empty() { String::new() } else { format!(" {label}") };
    format!(
        "{:>gutter$} | {line_text}\n{} | {}{}{label}\n",
        span.line + 1,
        " ".repeat(gutter),
        " ".repeat(span.col),
        mark.to_string().repeat(width),
    )
}
