use std::convert::identity;
use std::env::{var, VarError};
use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Result, Write};
use std::path::PathBuf;

use syn::{
    parse_quote, AttrStyle, Expr, ExprLit, Ident, Item, Lit, Member, Meta, MetaNameValue, Path,
};

const OPS_PATH: &str = "src/graph/ops";

fn main() {
    println!("cargo::rerun-if-changed={}", OPS_PATH);
    if Err(VarError::NotPresent) != var("CARGO_CFG_HYDROFLOW_GENERATE_DOCS") {
        if let Err(err) = generate_op_docs() {
            eprintln!("hydroflow_lang/build.rs error: {:?}", err);
        }
    }
}

fn generate_op_docs() -> Result<()> {
    for dir_entry in std::fs::read_dir(OPS_PATH)? {
        let dir_entry = dir_entry?;
        if !dir_entry.file_type()?.is_file()
            || "mod.rs" == dir_entry.file_name()
            || !dir_entry.file_name().to_string_lossy().ends_with(".rs")
        {
            continue;
        }
        let op_content = std::fs::read_to_string(dir_entry.path())?;
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

            let docgen_path = PathBuf::from_iter([
                std::env!("CARGO_MANIFEST_DIR"),
                "../docs/docgen",
                &*format!("{}.md", op_name),
            ]);
            eprintln!("{:?}", docgen_path);
            let mut docgen_write = BufWriter::new(File::create(docgen_path)?);
            writeln!(docgen_write, "<!-- GENERATED hydroflow_lang/build.rs -->")?;

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
                let Some("doc") = path.get_ident().map(Ident::to_string).as_deref() else {
                    continue;
                };
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
                        writeln!(docgen_write, "{}", DOCTEST_HYDROFLOW_SUFFIX)?;
                        // Output `doc_str` below.
                    } else if doc_str.trim() == "```hydroflow" {
                        in_hf_doctest = true;

                        writeln!(docgen_write, "```rust")?;
                        // py_udf special-cased.
                        if "py_udf" == op_name {
                            writeln!(docgen_write, "# #[cfg(feature = \"python\")]")?;
                        }
                        writeln!(docgen_write, "{}", DOCTEST_HYDROFLOW_PREFIX)?;
                        continue;
                    } else if doc_str.trim() == "```rustbook" {
                        writeln!(docgen_write, "```rust")?;
                        continue;
                    }
                }
                writeln!(docgen_write, "{}", doc_str)?;
            }
        }

        eprintln!("{:?}", dir_entry.file_name());
    }
    Ok(())
}

const DOCTEST_HYDROFLOW_PREFIX: &str = "\
# {
# let __rt = ::hydroflow::tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
# __rt.block_on(async { ::hydroflow::tokio::task::LocalSet::new().run_until(async {
# let mut __hf = ::hydroflow::hydroflow_syntax! {";
const DOCTEST_HYDROFLOW_SUFFIX: &str = "\
# };
# __hf.run_available();
# }).await})
# }";
