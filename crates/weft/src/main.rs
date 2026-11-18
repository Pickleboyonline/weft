//! The `weft` command line.
//!
//!   weft check <dir>            check a project
//!   weft explain effects <dir>  what every function and component does
//!   weft explain api <dir>      every query and command, with its types
//!   weft explain <code>         what a diagnostic means, with an example and the fix
//!   weft fmt [--check] <path>   format .weft files in the canonical style
//!   weft doctest [root]         check every ```weft block in the docs

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use weft_check::{check, load_project, original_span, snippet, Report, Source};
use weft_syntax::Severity;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let a: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match a.as_slice() {
        ["check", dir] => cmd_check(Path::new(dir)),
        ["explain", "effects", dir] => explain(Path::new(dir), |r| r.explain_effects()),
        ["explain", "api", dir] => explain(Path::new(dir), |r| r.explain_api()),
        ["explain", code] => explain_code(code),
        ["fmt", path] => fmt(Path::new(path), false),
        ["fmt", "--check", path] => fmt(Path::new(path), true),
        ["doctest"] => doctest(Path::new(".")),
        ["doctest", root] => doctest(Path::new(root)),
        _ => {
            eprintln!(
                "usage: weft check <dir> | weft explain (effects|api) <dir> | weft explain <code> | weft fmt [--check] <path> | weft doctest [root]"
            );
            ExitCode::from(2)
        }
    }
}

fn explain_code(code: &str) -> ExitCode {
    match weft_check::explain::lookup(code) {
        Some(e) => {
            println!("{}: {}\n\n{}", e.code, e.title, e.text);
            ExitCode::SUCCESS
        }
        None => {
            eprintln!("no diagnostic called `{code}`. The codes are:");
            for e in weft_check::explain::ENTRIES {
                eprintln!("  {:20} {}", e.code, e.title);
            }
            ExitCode::from(2)
        }
    }
}

fn fmt(path: &Path, check: bool) -> ExitCode {
    let mut files = Vec::new();
    if path.is_dir() {
        weft_files(path, &mut files);
    } else {
        files.push(path.to_path_buf());
    }
    files.sort();
    let mut changed = 0;
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let formatted = match weft_syntax::fmt::format(&text) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}: not formatted: {e}", f.display());
                continue;
            }
        };
        if formatted != text {
            changed += 1;
            if check {
                println!("{} would change", f.display());
            } else if std::fs::write(f, &formatted).is_ok() {
                println!("formatted {}", f.display());
            }
        }
    }
    if check && changed > 0 {
        println!("{changed} of {} files aren't formatted; run `weft fmt`", files.len());
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn weft_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            weft_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "weft") {
            out.push(p);
        }
    }
}

fn project(dir: &Path) -> Result<Vec<Source>, ExitCode> {
    load_project(dir).map_err(|e| {
        eprintln!("can't read {}: {e}", dir.display());
        ExitCode::from(2)
    })
}

fn cmd_check(dir: &Path) -> ExitCode {
    let sources = match project(dir) {
        Ok(s) => s,
        Err(c) => return c,
    };
    let report = check(&sources, true);
    let mut errors = 0;
    for (fi, d) in &report.diagnostics {
        let s = &sources[*fi];
        print!("{}", d.render(&s.path, &s.text));
        if d.severity == Severity::Error {
            errors += 1;
        }
    }
    println!("{} files, {errors} errors", sources.len());
    if errors > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

fn explain(dir: &Path, f: impl Fn(&Report) -> String) -> ExitCode {
    let sources = match project(dir) {
        Ok(s) => s,
        Err(c) => return c,
    };
    print!("{}", f(&check(&sources, true)));
    ExitCode::SUCCESS
}

/// A ```weft block from a Markdown file.
struct Block {
    file: PathBuf,
    /// 1-based line of the block's first code line in the Markdown file.
    line: usize,
    code: String,
}

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() && name != "node_modules" && name != "build" && !name.starts_with('.') {
            markdown_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            out.push(p);
        }
    }
}

fn blocks(file: &Path) -> Vec<Block> {
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_start();
        if t.starts_with("```weft") {
            let indent = lines[i].len() - t.len();
            let start = i + 1;
            let mut j = start;
            while j < lines.len() && !lines[j].trim_start().starts_with("```") {
                j += 1;
            }
            let code: String = lines[start..j]
                .iter()
                .map(|l| if l.len() >= indent && l[..indent].trim().is_empty() { &l[indent..] } else { l.trim_start() })
                .collect::<Vec<_>>()
                .join("\n");
            out.push(Block { file: file.to_path_buf(), line: start + 1, code: code + "\n" });
            i = j;
        }
        i += 1;
    }
    out
}

/// Lines the docs mark as failing: a line ending in `// error: …`, or the code
/// line before a comment line that starts with `// error:`.
fn expected_error_lines(code: &str) -> Vec<usize> {
    let lines: Vec<&str> = code.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if let Some(pos) = l.find("// error:") {
            if l[..pos].trim().is_empty() {
                let mut k = i;
                while k > 0 && lines[k - 1].trim_start().starts_with("//") {
                    k -= 1;
                }
                if k > 0 {
                    out.push(k - 1);
                }
            } else {
                out.push(i);
            }
        }
    }
    out
}

fn doctest(root: &Path) -> ExitCode {
    // The reference's snippets are about the todo-lists example, so its
    // definitions are in scope for every snippet.
    let prelude = load_project(&root.join("examples/todo-lists")).unwrap_or_default();
    let mut files = Vec::new();
    for d in ["website/docs", "docs", "examples"] {
        markdown_files(&root.join(d), &mut files);
    }
    files.sort();
    let mut total = 0;
    let mut failures = 0;
    let mut confirmed = 0;
    for f in &files {
        for b in blocks(f) {
            total += 1;
            let name = format!("{}:{}", b.file.display(), b.line);
            let mut sources = vec![snippet(name.clone(), b.code.clone())];
            sources.extend(prelude.iter().map(|p| weft_check::source(p.path.clone(), p.text.clone())));
            let report = check(&sources, false);
            let expected = expected_error_lines(&b.code);
            let mut seen = vec![false; expected.len()];
            let mut problems = Vec::new();
            for (fi, d) in &report.diagnostics {
                if *fi != 0 || d.severity != Severity::Error {
                    continue;
                }
                let sp = original_span(&sources[0], d.span);
                match expected.iter().position(|&l| l == sp.line) {
                    Some(k) => seen[k] = true,
                    None => problems.push(format!(
                        "  line {}: unexpected error[{}]: {}\n    | {}",
                        b.line + sp.line,
                        d.code,
                        d.message,
                        b.code.lines().nth(sp.line).unwrap_or("").trim()
                    )),
                }
            }
            for (k, &l) in expected.iter().enumerate() {
                if seen[k] {
                    confirmed += 1;
                } else {
                    problems.push(format!(
                        "  line {}: the docs say this fails, but the checker accepts it\n    | {}",
                        b.line + l,
                        b.code.lines().nth(l).unwrap_or("").trim()
                    ));
                }
            }
            if !problems.is_empty() {
                failures += 1;
                println!("{}:{}", b.file.display(), b.line);
                for p in problems {
                    println!("{p}");
                }
            }
        }
    }
    println!("\n{total} blocks, {failures} with problems, {confirmed} documented errors confirmed");
    if failures > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
