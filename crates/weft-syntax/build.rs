// Compiles the parser generated from tree-sitter-weft/grammar.js. Run
// `make grammar` after editing the grammar to regenerate src/parser.c.
fn main() {
    let dir = std::path::Path::new("../../tree-sitter-weft/src");
    cc::Build::new()
        .include(dir)
        .file(dir.join("parser.c"))
        .file(dir.join("scanner.c"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .compile("tree-sitter-weft");
    println!("cargo:rerun-if-changed=../../tree-sitter-weft/src/parser.c");
    println!("cargo:rerun-if-changed=../../tree-sitter-weft/src/scanner.c");
}
