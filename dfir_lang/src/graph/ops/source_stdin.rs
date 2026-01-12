use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: port number
///
/// `source_stdin` receives a Stream of lines from stdin
/// and emits each of the elements it receives downstream.
///
/// ```dfir
/// source_stdin()
///     -> map(|x| x.unwrap().to_uppercase())
///     -> for_each(|x| println!("{}", x));
/// ```
pub const SOURCE_STDIN: OperatorConstraints = OperatorConstraints {
    name: "source_stdin",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: true,
    has_singleton_output: false,
    flo_type: Some(FloType::Source),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   op_span,
                   ident,
                   ..
               },
               _| {
        let stream_ident = wc.make_ident("stream");
        let write_prologue = quote_spanned! {op_span=>
            #[allow(clippy::let_and_return, reason = "gives return value a self-documenting name")]
            let mut #stream_ident = {
                use #root::tokio::io::AsyncBufReadExt;
                let reader = #root::tokio::io::BufReader::new(#root::tokio::io::stdin());
                let stdin_lines = #root::tokio_stream::wrappers::LinesStream::new(reader.lines());
                stdin_lines
            };
        };
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::futures::stream::poll_fn(|_cx| {
                match #root::futures::stream::Stream::poll_next(::std::pin::Pin::new(&mut #stream_ident), &mut ::std::task::Context::from_waker(&#context.waker())) {
                    ::std::task::Poll::Ready(maybe) => ::std::task::Poll::Ready(maybe),
                    ::std::task::Poll::Pending => ::std::task::Poll::Ready(::std::option::Option::None),
                }
            });
        };
        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
