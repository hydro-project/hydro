use std::collections::BTreeSet;
use std::fs::read_dir;
use std::process::Command;

use insta::assert_snapshot;

/// Bit of a jank test, runs `cargo run -p dfir --example <EXAMPLE>` for all the
/// `example_*.rs` examples and uses `insta` to snapshot tests the stdout.
#[test]
fn test_all() {
    let examples_files = read_dir("examples/")
        .unwrap()
        .flat_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|file_type| file_type.is_file()))
        .map(|entry| entry.file_name())
        .map(|filename| filename.into_string().unwrap())
        .filter(|filename| filename.starts_with("example_") && filename.ends_with(".rs"))
        .collect::<BTreeSet<_>>();

    for example_file in examples_files {
        let name = example_file.strip_suffix(".rs").unwrap();

        let output = Command::new("cargo")
            .args(["run", "-p", env!("CARGO_PKG_NAME"), "--example"])
            .arg(name)
            .output()
            .expect("Failed to run example.");
        let output = String::from_utf8_lossy(&output.stdout);
        assert_snapshot!(name, output);
    }
}
