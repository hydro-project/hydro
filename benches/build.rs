use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const NUM_OPS: usize = 20;

pub fn main() {
    for v2 in [false, true] {
        if let Err(err) = fork_join(v2) {
            eprintln!("benches/build.rs error: {:?}", err);
        }
    }
}

pub fn fork_join(v2: bool) -> std::io::Result<()> {
    let path = PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "benches",
        if v2 { "fork_join_2.hf" } else { "fork_join.hf" },
    ]);
    let file = File::create(path)?;
    let mut write = BufWriter::new(file);

    writeln!(write, "dfir_syntax! {{")?;
    writeln!(
        write,
        "a0 = source_iter({}) -> tee();",
        if v2 { "vals()" } else { "0..NUM_INTS" }
    )?;
    for i in 0..NUM_OPS {
        if i > 0 {
            writeln!(write, "a{} = union() -> tee();", i)?;
        }
        if v2 {
            writeln!(
                write,
                "a{0} -> filter(|x| (x >> {0}) & 0b1 == 0) -> a{1};",
                i,
                i + 1
            )?;
            writeln!(
                write,
                "a{0} -> filter(|x| (x >> {0}) & 0b1 == 1) -> a{1};",
                i,
                i + 1
            )?;
        } else {
            writeln!(write, "a{} -> filter(|x| x % 2 == 0) -> a{};", i, i + 1)?;
            writeln!(write, "a{} -> filter(|x| x % 2 == 1) -> a{};", i, i + 1)?;
        }
    }
    writeln!(
        write,
        "a{} = union() -> for_each(|x| {{ black_box(x); }});",
        NUM_OPS
    )?;
    writeln!(write, "}}")?;

    write.flush()?;

    Ok(())
}
