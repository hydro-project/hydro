use quote::quote_spanned;
use syn::parse_quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs, make_missing_runtime_msg,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: An [`AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html)`<`[`Path`](https://doc.rust-lang.org/nightly/std/path/struct.Path.html)`>`
/// > for a file to read.
///
/// Reads the referenced file one line at a time. The line will NOT include the line ending.
///
/// Will panic if the file could not be read, or if the file contains bytes that are not valid UTF-8.
///
/// ```dfir
/// source_file("Cargo.toml") -> for_each(|line| println!("{}", line));
/// ```
pub const SOURCE_FILE: OperatorConstraints = OperatorConstraints {
    name: "source_file",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: true,
    has_singleton_output: false,
    flo_type: Some(FloType::Source),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   op_name,
                   arguments,
                   ..
               },
               diagnostics| {
        let filename_arg = &arguments[0];

        let ident_filelines = wc.make_ident("filelines");

        let missing_runtime_msg = make_missing_runtime_msg(op_name);

        let write_prologue = quote_spanned! {op_span=>
            let #ident_filelines = {
                // Could use `let file = #root::tokio::fs::File::open(#arguments).await` directly,
                // but only if we're in an async fn, which we can't know (right now).
                let file = ::std::fs::File::open(#filename_arg).expect("Failed to open file for reading");
                let file = #root::tokio::fs::File::from_std(file);
                let bufread = #root::tokio::io::BufReader::new(file);
                let lines = #root::tokio::io::AsyncBufReadExt::lines(bufread);
                #root::tokio_stream::wrappers::LinesStream::new(lines)
            };
        };
        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned!(op_span=> #ident_filelines),
            ..wc.clone()
        };

        let OperatorWriteOutput {
            write_prologue: write_prologue_stream,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::source_stream::SOURCE_STREAM.write_fn)(&wc, diagnostics)?;

        let write_prologue = quote_spanned! {op_span=>
            #write_prologue
            #write_prologue_stream
        };
        let write_iterator = quote_spanned! {op_span=>
            ::std::debug_assert!(#root::tokio::runtime::Handle::try_current().is_ok(), #missing_runtime_msg);
            #write_iterator
            // Unwrap each line. Will panic if invalid utf-8.
            let #ident = #ident.map(|result| result.unwrap());
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
