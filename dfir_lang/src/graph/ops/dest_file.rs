use quote::quote_spanned;
use syn::parse_quote_spanned;

use super::{
    make_missing_runtime_msg, OperatorCategory, OperatorConstraints,
    OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: (1) An [`AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html)`<`[`Path`](https://doc.rust-lang.org/nightly/std/path/struct.Path.html)`>`
/// > for a file to write to, and (2) a bool `append`.
///
/// Consumes `String`s by writing them as lines to a file. The file will be created if it doesn't
/// exist. Lines will be appended to the file if `append` is true, otherwise the file will be
/// truncated before lines are written.
///
/// Note this operator must be used within a Tokio runtime.
///
/// ```dfir
/// source_iter(1..=10) -> map(|n| format!("Line {}", n)) -> dest_file("dest.txt", false);
/// ```
pub const DEST_FILE: OperatorConstraints = OperatorConstraints {
    name: "dest_file",
    categories: &[OperatorCategory::Sink],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_0,
    soft_range_out: RANGE_0,
    num_args: 2,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   op_name,
                   arguments,
                   ..
               },
               diagnostics| {
        let filename_arg = &arguments[0];
        let append_arg = &arguments[1];

        let ident_filesink = wc.make_ident("filesink");

        let missing_runtime_msg = make_missing_runtime_msg(op_name);

        let write_prologue = quote_spanned! {op_span=>
            let #ident_filesink = {
                // Could use `#root::tokio::fs::OpenOptions` but only if we're in an async fn,
                // which we can't know (right now)
                let append = #append_arg;
                let file = ::std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(append)
                    .truncate(!append)
                    .open(#filename_arg)
                    .expect("Failed to open file for writing");
                let file = #root::tokio::fs::File::from_std(file);
                let bufwrite = #root::tokio::io::BufWriter::new(file);
                let codec = #root::tokio_util::codec::LinesCodec::new();
                #root::tokio_util::codec::FramedWrite::new(bufwrite, codec)
            };
        };
        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned!(op_span=> #ident_filesink),
            ..wc.clone()
        };

        let OperatorWriteOutput {
            write_prologue: write_prologue_sink,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::dest_sink::DEST_SINK.write_fn)(&wc, diagnostics)?;

        let write_prologue = quote_spanned! {op_span=>
            #write_prologue
            #write_prologue_sink
        };
        let write_iterator = quote_spanned! {op_span=>
            ::std::debug_assert!(#root::tokio::runtime::Handle::try_current().is_ok(), #missing_runtime_msg);
            #write_iterator
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
