//! The formatter: idempotent, and it keeps what the code means.

use std::path::{Path, PathBuf};

fn weft_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            weft_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "weft") {
            out.push(p);
        }
    }
}

fn shape(src: &str) -> String {
    // The parse tree without positions: formatting must not change it.
    let s = weft_syntax::parse(src).sexp;
    s
}

#[test]
fn formatting_is_idempotent_and_keeps_meaning() {
    let mut files = Vec::new();
    weft_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"), &mut files);
    assert!(!files.is_empty());
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        let once = weft_syntax::fmt::format(&src).unwrap();
        let twice = weft_syntax::fmt::format(&once).unwrap();
        assert_eq!(once, twice, "{} isn't stable", f.display());
        assert_eq!(shape(&src), shape(&once), "{} changed meaning", f.display());
    }
}

#[test]
fn messy_input_comes_out_canonical() {
    let messy = "items_in   =   Query(|list:Id(TodoList)|->List(Item){\n        items.filter( |i|i.list==list )\n            .sort_by( .created )\n})\n";
    let want = "items_in = Query(|list: Id(TodoList)| -> List(Item) {\n  items.filter(|i| i.list == list)\n    .sort_by(.created)\n})\n";
    assert_eq!(weft_syntax::fmt::format(messy).unwrap(), want);
}
