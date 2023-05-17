//! Build script to generate operator book docs.

use std::env::VarError;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use hydroflow_lang::graph::ops::{PortListSpec, OPERATORS};
use hydroflow_lang::graph::PortIndexValue;
use quote::ToTokens;

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

fn update_book() -> Result<(), Box<dyn Error>> {
    let mut ops: Vec<_> = OPERATORS.iter().collect();
    ops.sort_by_key(|op| op.name);

    let mut write = book_file_writer(FILENAME)?;
    writeln!(
        write,
        "<!-- GENERATED {:?} -->",
        file!().replace(std::path::MAIN_SEPARATOR, "/")
    )?;
    writeln!(write, "{}", PREFIX)?;
    writeln!(write)?;
    writeln!(write, "| All Operators | | | |")?;
    writeln!(write, "| --- | --- | --- | --- |")?;
    for op_chunk in ops.chunks(4) {
        write!(write, "|")?;
        for op in op_chunk {
            write!(write, " [`{0}`](#{0}) |", op.name)?;
        }
        writeln!(write)?;
    }
    writeln!(write)?;
    for &op in ops.iter() {
        writeln!(write, "## `{}`", op.name)?;

        writeln!(write, "| Inputs | Syntax | Outputs | Flow |")?;
        writeln!(write, "| ------ | ------ | ------- | ---- |")?;
        let metadata_str = format!(
            "| <span title={:?}>{}</span> | `{}{}({}){}` | <span title={:?}>{}</span> |",
            op.hard_range_inn.human_string(),
            op.soft_range_inn.human_string(),
            if op.soft_range_inn.contains(&0) {
                ""
            } else if op.soft_range_inn.contains(&1) {
                "-> "
            } else {
                "-> [<input_port>]"
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
            } else if op.soft_range_out.contains(&1) {
                " ->"
            } else {
                "[<output_port>] ->"
            },
            op.hard_range_out.human_string(),
            op.soft_range_out.human_string(),
        );

        let mut blocking = false;
        let input_port_names = op.ports_inn.map(|ports_inn_fn| (ports_inn_fn)());
        let input_str_maybe = match input_port_names {
            None => {
                blocking = (op.input_delaytype_fn)(&PortIndexValue::Elided(None)).is_some();
                None
            }
            Some(PortListSpec::Fixed(port_names)) => Some(format!(
                "> Input port names: {}  ",
                port_names
                    .into_iter()
                    .map(|idx| {
                        let port_ix = idx.clone().into();
                        let flow_str = if (op.input_delaytype_fn)(&port_ix).is_some() {
                            blocking = true;
                            "blocking"
                        } else {
                            "streaming"
                        };
                        format!("`{}` ({}), ", idx.into_token_stream(), flow_str)
                    })
                    .collect::<String>()
                    .strip_suffix(", ")
                    .unwrap_or("&lt;EMPTY&gt;")
            )),
            Some(PortListSpec::Variadic) => {
                Some("> Input port names: Variadic, as specified in arguments.".to_string())
            }
        };
        let mut output_str_maybe = None;
        if let Some(f) = op.ports_out {
            output_str_maybe = Some(match (f)() {
                PortListSpec::Fixed(port_names) => format!(
                    "> Output port names: {}  ",
                    port_names
                        .into_iter()
                        .map(|idx| format!("`{}`, ", idx.into_token_stream()))
                        .collect::<String>()
                        .strip_suffix(", ")
                        .unwrap_or("&lt;EMPTY&gt;")
                ),
                PortListSpec::Variadic => {
                    "> Output port names: Variadic, as specified in arguments.".to_string()
                }
            })
        }
        let flow_str = if blocking { "Blocking" } else { "Streaming" };
        writeln!(write, "{}{} |", metadata_str, flow_str)?;
        writeln!(write)?;
        if let Some(input_str) = input_str_maybe {
            writeln!(write, "{}", input_str)?;
        }
        if let Some(output_str) = output_str_maybe {
            writeln!(write, "{}", output_str)?;
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

fn main() -> Result<(), Box<dyn Error>> {
    // Only try to update the book if the output directory exists.
    if book_file(FILENAME)?
        .parent()
        .map_or(false, |dir| dir.is_dir())
    {
        update_book()?;
    }
    Ok(())
}

const PREFIX: &str = "\
# Hydroflow's Operators

In our previous examples we made use of some of Hydroflow's operators.
Here we document each operator in more detail. Most of these operators
are based on the Rust equivalents for iterators; see the [Rust documentation](https://doc.rust-lang.org/std/iter/trait.Iterator.html).";
