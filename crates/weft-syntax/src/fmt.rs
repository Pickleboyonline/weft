//! `weft fmt`: the one canonical style, with no options.
//!
//! The formatter works on the concrete tree, so comments are kept. Line breaks
//! are the author's (they end statements, so they're part of the meaning), up
//! to one blank line in a row. Everything else is decided here:
//!
//! - indentation: two spaces per open bracket, plus two for a line that
//!   continues the one before (a `.method` chain, or after an operator);
//! - spacing inside a line: one space between tokens, none inside `(…)` and
//!   `[…]`, after `.`, before `,`, `?` and `:` (except ` : ` in signatures);
//! - alignment, as `gofmt` does it: consecutive one-line bindings align their
//!   `=`, consecutive `match` arms their `->`, record fields their types and
//!   defaults, and consecutive trailing comments their `//`.

use std::collections::HashMap;

use tree_sitter::{Node, Parser};

#[derive(Clone, Debug)]
struct Token {
    start: usize,
    row: usize,
    end_row: usize,
    text: String,
    /// The node kind for named leaves (`identifier`), the text for the rest.
    kind: String,
    parent: String,
    /// For `|`: opens or closes a lambda's parameters.
    lambda_bar: Option<bool>,
}

/// A statement whose parts can be aligned with its neighbours.
struct Alignable {
    kind: &'static str,
    one_line: bool,
    /// Byte offsets of the tokens to align, in order.
    anchors: Vec<usize>,
}

pub fn format(src: &str) -> Result<String, String> {
    let mut parser = Parser::new();
    parser.set_language(&crate::LANGUAGE.into()).map_err(|e| e.to_string())?;
    let tree = parser.parse(src, None).ok_or("couldn't parse")?;
    let root = tree.root_node();
    if root.has_error() {
        return Err("it has syntax errors".into());
    }
    let mut tokens = Vec::new();
    let mut aligns = HashMap::new();
    collect(root, src, "", &mut tokens, &mut aligns);
    tokens.sort_by_key(|t| t.start);
    Ok(render(&tokens, &aligns))
}

const OPAQUE: &[&str] = &["text", "css_body", "line_comment", "block_comment", "number"];

fn collect(n: Node, src: &str, parent: &str, out: &mut Vec<Token>, aligns: &mut HashMap<usize, Alignable>) {
    let kind = n.kind();
    if let Some(a) = alignable(n) {
        aligns.insert(n.start_byte(), a);
    }
    if n.child_count() == 0 || OPAQUE.contains(&kind) {
        if n.start_byte() == n.end_byte() {
            return;
        }
        out.push(Token {
            start: n.start_byte(),
            row: n.start_position().row,
            end_row: n.end_position().row,
            text: src[n.byte_range()].to_string(),
            kind: if n.is_named() { kind.to_string() } else { src[n.byte_range()].to_string() },
            parent: parent.to_string(),
            lambda_bar: None,
        });
        return;
    }
    let mut cur = n.walk();
    let children: Vec<Node> = n.children(&mut cur).collect();
    let bars: Vec<usize> =
        if kind == "lambda" { children.iter().filter(|c| c.kind() == "|").map(|c| c.start_byte()).collect() } else { vec![] };
    for c in children {
        let before = out.len();
        collect(c, src, kind, out, aligns);
        if kind == "lambda" && c.kind() == "|" && out.len() > before {
            out.last_mut().unwrap().lambda_bar = Some(bars.first() == Some(&c.start_byte()));
        }
    }
}

fn alignable(n: Node) -> Option<Alignable> {
    let one_line = n.start_position().row == n.end_position().row;
    let direct = |what: &str| -> Option<usize> {
        let mut c = n.walk();
        let found = n.children(&mut c).find(|k| k.kind() == what).map(|k| k.start_byte());
        found
    };
    // Declarations align; statements inside function bodies don't.
    let declaration_level = |n: Node| match n.parent() {
        Some(p) if matches!(p.kind(), "source_file" | "record_declaration") => true,
        Some(p) if p.kind() == "block" => {
            p.parent().is_some_and(|g| matches!(g.kind(), "component_declaration" | "handler_declaration"))
        }
        _ => false,
    };
    match n.kind() {
        "binding" if declaration_level(n) => {
            Some(Alignable { kind: "assign", one_line, anchors: direct("=").into_iter().collect() })
        }
        "state_declaration" => Some(Alignable { kind: "assign", one_line, anchors: direct("=").into_iter().collect() }),
        // A record literal written one field per line aligns its values.
        "field" => Some(Alignable {
            kind: "field-value",
            one_line,
            anchors: n.child_by_field_name("value").map(|v| v.start_byte()).into_iter().collect(),
        }),
        "match_arm" => Some(Alignable { kind: "arm", one_line, anchors: direct("->").into_iter().collect() }),
        "record_field" => {
            let mut anchors: Vec<usize> = n.child_by_field_name("type").map(|t| t.start_byte()).into_iter().collect();
            anchors.extend(direct("="));
            Some(Alignable { kind: "field", one_line, anchors })
        }
        _ => None,
    }
}

fn is_open(t: &str) -> bool {
    matches!(t, "(" | "[" | "{")
}
fn is_close(t: &str) -> bool {
    matches!(t, ")" | "]" | "}")
}

const BINARY: &[&str] = &["+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">=", "and", "or", "=", "->", "..<", "..="];

/// Whether there's a space between two tokens on the same line.
fn space(a: &Token, b: &Token) -> bool {
    let (p, n) = (a.kind.as_str(), b.kind.as_str());
    if matches!(n, ")" | "]" | "," | "?") {
        return false;
    }
    if n == "." {
        // `.created` as an argument keeps its space; `x.y` doesn't.
        return b.parent == "accessor" && !is_open(p);
    }
    if matches!(p, "(" | "[" | ".") {
        return false;
    }
    if p == ".." || matches!(n, "..<" | "..=") || matches!(p, "..<" | "..=") {
        return false;
    }
    if n == ":" {
        return matches!(b.parent.as_str(), "signature" | "operation" | "constraint" | "alias");
    }
    if p == "-" && a.parent == "unary_expression" {
        return false;
    }
    if n == "|" && b.lambda_bar == Some(false) {
        return false;
    }
    if p == "|" && a.lambda_bar == Some(true) {
        return false;
    }
    if n == "(" {
        return !matches!(
            b.parent.as_str(),
            "arguments" | "parameter_list" | "type_application" | "tag_type" | "tag_pattern" | "alias"
        );
    }
    if n == "}" && p == "{" {
        return false;
    }
    // A style block's CSS starts on its own line and ends before the `}`.
    if n == "css_body" || p == "css_body" {
        return false;
    }
    if n == "{" && is_open(p) {
        return false;
    }
    true
}

struct Line {
    indent: usize,
    blank_before: bool,
    /// The text, and where each token starts in it.
    text: String,
    starts: Vec<(usize, usize)>,
    first: usize,
    trailing_comment: Option<usize>,
}

fn render(tokens: &[Token], aligns: &HashMap<usize, Alignable>) -> String {
    // Group tokens into lines, keeping the author's line breaks.
    let mut groups: Vec<Vec<&Token>> = Vec::new();
    let mut last_end_row: Option<usize> = None;
    let mut blanks: Vec<bool> = Vec::new();
    for t in tokens {
        match last_end_row {
            Some(r) if t.row <= r => groups.last_mut().unwrap().push(t),
            _ => {
                blanks.push(last_end_row.is_some_and(|r| t.row > r + 1));
                groups.push(vec![t]);
            }
        }
        last_end_row = Some(t.end_row);
    }

    // Each open bracket remembers the line that opened it. A line is
    // indented once per *line* with open brackets, so `Command(|…| {` on one
    // line adds one level, not two.
    let mut open: Vec<usize> = Vec::new();
    let level = |open: &[usize]| {
        let mut lines: Vec<usize> = open.to_vec();
        lines.dedup();
        lines.len()
    };
    let mut lines: Vec<Line> = Vec::new();
    let mut prev_last: Option<&Token> = None;
    for (gi, g) in groups.iter().enumerate() {
        let first = g[0];
        // Brackets closed at the start of this line don't count for it.
        let mut stack = open.clone();
        for t in g.iter() {
            if is_close(&t.kind) {
                stack.pop();
            } else {
                break;
            }
        }
        let mut indent = level(&stack);
        let continues = first.kind == "." && first.parent != "accessor"
            || prev_last.is_some_and(|p| BINARY.contains(&p.kind.as_str()) && p.kind != "->");
        if continues {
            indent += 1;
        }
        let mut text = String::new();
        let mut starts = Vec::new();
        let mut trailing = None;
        for (i, t) in g.iter().enumerate() {
            if i > 0 && space(g[i - 1], t) {
                text.push(' ');
            }
            if t.kind == "line_comment" && i > 0 {
                trailing = Some(text.len());
            }
            starts.push((t.start, text.len()));
            if t.kind == "css_body" {
                text.push_str(&css(&t.text, level(&open) + 1));
            } else {
                text.push_str(&t.text);
            }
        }
        for t in g {
            if is_open(&t.kind) {
                open.push(gi);
            } else if is_close(&t.kind) {
                open.pop();
            }
        }
        prev_last = g.iter().rev().find(|t| !matches!(t.kind.as_str(), "line_comment" | "block_comment")).copied();
        lines.push(Line { indent, blank_before: blanks[gi], text, starts, first: first.start, trailing_comment: trailing });
    }

    align(&mut lines, aligns);
    align_comments(&mut lines);

    let mut out = String::new();
    for l in &lines {
        if l.blank_before {
            out.push('\n');
        }
        out.push_str(&"  ".repeat(l.indent));
        out.push_str(l.text.trim_end());
        out.push('\n');
    }
    out
}

/// Re-indents the CSS in a `style { … }` block.
fn css(body: &str, indent: usize) -> String {
    let mut out = String::from("\n");
    let mut level = 0usize;
    for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with('}') {
            level = level.saturating_sub(1);
        }
        out.push_str(&"  ".repeat(indent + level));
        out.push_str(line);
        out.push('\n');
        level += line.matches('{').count();
        level = level.saturating_sub(line.matches('}').count().saturating_sub(line.starts_with('}') as usize));
    }
    out.push_str(&"  ".repeat(indent.saturating_sub(1)));
    out
}

/// Aligns anchors across runs of consecutive one-line statements of one kind.
fn align(lines: &mut [Line], aligns: &HashMap<usize, Alignable>) {
    let key = |l: &Line| aligns.get(&l.first).filter(|a| a.one_line && !a.anchors.is_empty()).map(|a| a.kind);
    let mut i = 0;
    while i < lines.len() {
        let Some(k) = key(&lines[i]) else {
            i += 1;
            continue;
        };
        let mut j = i + 1;
        while j < lines.len() && !lines[j].blank_before && lines[j].indent == lines[i].indent && key(&lines[j]) == Some(k) {
            j += 1;
        }
        let max_anchors = (i..j).map(|n| aligns[&lines[n].first].anchors.len()).max().unwrap_or(0);
        if j - i >= 2 {
            for anchor in 0..max_anchors {
                // Only the lines that have this anchor take part.
                let members: Vec<(usize, usize)> = (i..j)
                    .filter_map(|n| {
                        let a = *aligns[&lines[n].first].anchors.get(anchor)?;
                        lines[n].starts.iter().find(|(b, _)| *b == a).map(|(_, c)| (n, *c))
                    })
                    .collect();
                if members.len() < 2 {
                    continue;
                }
                let target = members.iter().map(|(n, c)| lines[*n].text[..*c].trim_end().len() + 1).max().unwrap_or(0);
                for (n, c) in members {
                    let head = lines[n].text[..c].trim_end().to_string();
                    let pad = target - head.len();
                    let shift = (head.len() + pad) as isize - c as isize;
                    lines[n].text = format!("{head}{}{}", " ".repeat(pad), &lines[n].text[c..]);
                    for s in lines[n].starts.iter_mut().filter(|(_, col)| *col >= c) {
                        s.1 = (s.1 as isize + shift) as usize;
                    }
                    if let Some(tc) = lines[n].trailing_comment.as_mut() {
                        if *tc >= c {
                            *tc = (*tc as isize + shift) as usize;
                        }
                    }
                }
            }
        }
        i = j;
    }
}

/// Aligns the `//` of trailing comments on consecutive lines.
fn align_comments(lines: &mut [Line]) {
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trailing_comment.is_none() {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < lines.len() && !lines[j].blank_before && lines[j].trailing_comment.is_some() {
            j += 1;
        }
        let width = |l: &Line| 2 * l.indent + l.text[..l.trailing_comment.unwrap()].trim_end().len();
        let target = (i..j).map(|n| width(&lines[n])).max().unwrap_or(0) + 2;
        for n in i..j {
            let c = lines[n].trailing_comment.unwrap();
            let head = lines[n].text[..c].trim_end().to_string();
            let pad = target - 2 * lines[n].indent - head.len();
            lines[n].text = format!("{head}{}{}", " ".repeat(pad), &lines[n].text[c..]);
        }
        i = j;
    }
}
