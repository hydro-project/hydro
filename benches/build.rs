use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const NUM_OPS: usize = 20;

pub fn main() {
    if let Err(err) = fork_join() {
        eprintln!("benches/build.rs error: {:?}", err);
    }
}

pub fn fork_join() -> std::io::Result<()> {
    let path = PathBuf::from_iter([
        env!("CARGO_MANIFEST_DIR"),
        "benches",
        &format!("fork_join_{}.hf", NUM_OPS),
    ]);
    let file = File::create(path)?;
    let mut write = BufWriter::new(file);

    writeln!(write, "dfir_syntax! {{")?;
    writeln!(write, "a0 = source_iter(0..NUM_INTS) -> tee();")?;
    for i in 0..NUM_OPS {
        if i > 0 {
            writeln!(write, "a{} = union() -> tee();", i)?;
        }
        writeln!(write, "a{} -> filter(|x| x % 2 == 0) -> a{};", i, i + 1)?;
        writeln!(write, "a{} -> filter(|x| x % 2 == 1) -> a{};", i, i + 1)?;
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
