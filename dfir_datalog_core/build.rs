use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-changed=src");
    rust_sitter_tool::build_parsers(&PathBuf::from("src/lib.rs"));
}
