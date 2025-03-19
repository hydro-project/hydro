use std::convert::identity;
use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Result, Write};
use std::path::PathBuf;

use rustc_version::{Channel, version_meta};
use syn::{AttrStyle, Expr, ExprLit, Item, Lit, Member, Meta, MetaNameValue, Path, parse_quote};

const OPS_PATH: &str = "src/graph/ops";

fn main() {
    const DFIR_GENERATE_DOCS: &str = "DFIR_GENERATE_DOCS";

    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed={}", OPS_PATH);
    println!("cargo::rerun-if-env-changed={}", DFIR_GENERATE_DOCS);

    println!("cargo::rustc-check-cfg=cfg(nightly)");
    if matches!(
        version_meta().map(|meta| meta.channel),
        Ok(Channel::Nightly)
    ) {
        println!("cargo:rustc-cfg=nightly");
    }

    if std::env::var_os(DFIR_GENERATE_DOCS).is_some() {
        if let Err(err) = generate_op_docs() {
            eprintln!("{} error: {:?}", file!(), err);
        }
    }
}

fn generate_op_docs() -> Result<()> {
    let docgen_dir = PathBuf::from_iter([std::env!("CARGO_MANIFEST_DIR"), "../docs/docgen"]);
    // Clear all existing docs.
    for old_doc in std::fs::read_dir(&docgen_dir)? {
        let old_entry = old_doc?;
        if old_entry.file_type()?.is_file()
            && old_entry.file_name().to_string_lossy().ends_with(".md")
        {
            std::fs::remove_file(old_entry.path())?;
        }
    }

    for op_file in std::fs::read_dir(OPS_PATH)? {
        let op_file = op_file?;
        if !op_file.file_type()?.is_file()
            || "mod.rs" == op_file.file_name()
            || !op_file.file_name().to_string_lossy().ends_with(".rs")
        {
            continue;
        }
        let op_content = std::fs::read_to_string(op_file.path())?;
        let op_parsed = syn::parse_file(&op_content)
            .map_err(|syn_err| Error::new(ErrorKind::InvalidData, syn_err))?;

        for item in op_parsed.items {
            let Item::Const(item_const) = item else {
                continue;
            };
            let Expr::Struct(expr_struct) = *item_const.expr else {
                continue;
            };
            if identity::<Path>(parse_quote!(OperatorConstraints)) != expr_struct.path {
                continue;
            }

            let name_field = expr_struct
                .fields
                .iter()
                .find(|&field_value| identity::<Member>(parse_quote!(name)) == field_value.member)
                .expect("Expected `name` field not found.");
            let Expr::Lit(ExprLit {
                lit: Lit::Str(op_name),
                ..
            }) = &name_field.expr
            else {
                panic!("Unexpected non-literal or non-str `name` field value.")
            };
            let op_name = op_name.value();

            let docgen_file = docgen_dir.join(format!("{}.md", op_name));
            eprintln!("{:?}", docgen_file);
            let mut docgen_write = BufWriter::new(File::create(docgen_file)?);
            writeln!(docgen_write, "<!-- GENERATED BY {} -->", file!())?;

            let mut in_hf_doctest = false;
            for attr in item_const.attrs.iter() {
                let AttrStyle::Outer = attr.style else {
                    continue;
                };
                let Meta::NameValue(MetaNameValue {
                    path,
                    eq_token: _,
                    value,
                }) = &attr.meta
                else {
                    continue;
                };
                if !path.is_ident("doc") {
                    continue;
                }
                let Expr::Lit(ExprLit { attrs: _, lit }) = value else {
                    continue;
                };
                let Lit::Str(doc_lit_str) = lit else {
                    continue;
                };
                // At this point we know we have a `#[doc = "..."]`.
                let doc_str = doc_lit_str.value();
                let doc_str = doc_str.strip_prefix(' ').unwrap_or(&*doc_str);
                if doc_str.trim_start().starts_with("```") {
                    if in_hf_doctest {
                        in_hf_doctest = false;
                        writeln!(docgen_write, "{}", DOCTEST_SUFFIX)?;
                        // Output `doc_str` below.
                    } else if doc_str.trim() == "```dfir" {
                        in_hf_doctest = true;

                        writeln!(docgen_write, "```rust")?;
                        // py_udf special-cased.
                        if "py_udf" == op_name {
                            writeln!(docgen_write, "# #[cfg(feature = \"python\")]")?;
                        }
                        writeln!(docgen_write, "{}", DOCTEST_PREFIX)?;
                        continue;
                    } else if doc_str.trim() == "```rustbook" {
                        writeln!(docgen_write, "```rust")?;
                        continue;
                    }
                }
                writeln!(docgen_write, "{}", doc_str)?;
            }
        }

        eprintln!("{:?}", op_file.file_name());
    }
    Ok(())
}

const DOCTEST_PREFIX: &str = "\
# {
# let __rt = ::dfir_rs::tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
# __rt.block_on(async { ::dfir_rs::tokio::task::LocalSet::new().run_until(async {
# let mut __hf = ::dfir_rs::dfir_syntax! {";
const DOCTEST_SUFFIX: &str = "\
# };
# __hf.run_available();
# }).await})
# }";
