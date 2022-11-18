//! Build script to generate operator book docs.

use std::env::VarError;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use quote::ToTokens;

use hydroflow_lang::graph::ops::OPERATORS;

const FILENAME: &str = "surface_ops.gen.md";

fn book_file(filename: impl AsRef<Path>) -> Result<PathBuf, VarError> {
    let mut pathbuf = PathBuf::new();
    pathbuf.push(std::env::var("CARGO_MANIFEST_DIR")?);
    pathbuf.push("../book/");
    pathbuf.push(filename);
    Ok(pathbuf)
}

fn book_file_writer(filename: impl AsRef<Path>) -> Result<BufWriter<File>, Box<dyn Error>> {
    let pathbuf = book_file(filename)?;
    Ok(BufWriter::new(File::create(pathbuf)?))
}

fn write_operator_docgen(op_name: &str, mut write: &mut impl Write) -> std::io::Result<()> {
    let doctest_path = PathBuf::from_iter([
        std::env!("CARGO_MANIFEST_DIR"),
        "../book/docgen",
        &*format!("{}.md", op_name),
    ]);
    let mut read = BufReader::new(File::open(doctest_path)?);
    std::io::copy(&mut read, &mut write)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut write = book_file_writer(FILENAME)?;
    writeln!(
        write,
        "<!-- GENERATED {:?} -->",
        file!().replace(std::path::MAIN_SEPARATOR, "/")
    )?;
    writeln!(write, "{}", PREFIX)?;
    for op in OPERATORS {
        writeln!(write, "## `{}`", op.name)?;

        writeln!(write, "| Inputs | Syntax | Outputs |")?;
        writeln!(write, "| ------ | ------ | ------- |")?;
        writeln!(
            write,
            "| <span title={:?}>{}</span> | `{}{}({}){}` | <span title={:?}>{}</span> |",
            op.hard_range_inn.human_string(),
            op.soft_range_inn.human_string(),
            if op.soft_range_inn.contains(&0) {
                ""
            } else {
                "-> "
            },
            op.name,
            ('A'..)
                .take(op.num_args)
                .map(|c| format!("{}, ", c))
                .collect::<String>()
                .strip_suffix(", ")
                .unwrap_or(""),
            if op.soft_range_out.contains(&0) {
                ""
            } else {
                " ->"
            },
            op.hard_range_out.human_string(),
            op.soft_range_out.human_string(),
        )?;
        writeln!(write)?;

        if let Some(f) = op.ports_inn {
            writeln!(
                write,
                "> Input port names: {}  ",
                (f)()
                    .into_iter()
                    .map(|idx| format!("`{}`, ", idx.into_token_stream()))
                    .collect::<String>()
                    .strip_suffix(", ")
                    .unwrap_or("&lt;EMPTY&gt;")
            )?;
        }
        if let Some(f) = op.ports_out {
            writeln!(
                write,
                "> Output port names: {}  ",
                (f)()
                    .into_iter()
                    .map(|idx| format!("`{}`, ", idx.into_token_stream()))
                    .collect::<String>()
                    .strip_suffix(", ")
                    .unwrap_or("&lt;EMPTY&gt;")
            )?;
        }
        writeln!(write)?;

        if let Err(err) = write_operator_docgen(op.name, &mut write) {
            eprintln!("Docgen error: {}", err);
        }
        writeln!(write)?;
        writeln!(write)?;
    }

    Ok(())
}

const PREFIX: &str = "\
# Hydroflow's Built-in Operators

In our previous examples we made use of some of Hydroflow's built-in operators.
Here we document each operators in more detail. Most of these operators
are based on the Rust equivalents for iterators; see the [Rust documentation](https://doc.rust-lang.org/std/iter/trait.Iterator.html).";
