use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: The receive end of a tokio channel
///
/// Given a [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)
/// created in Rust code, `source_stream`
/// is passed the receive endpoint of the channel and emits each of the
/// elements it receives downstream.
///
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<&str>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_stream(input_recv) -> map(|x| x.to_uppercase())
///         -> for_each(|x| println!("{}", x));
/// };
/// input_send.send("Hello").unwrap();
/// input_send.send("World").unwrap();
/// flow.run_available();
/// ```
pub const SOURCE_STREAM: OperatorConstraints = OperatorConstraints {
    name: "source_stream",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: true,
    flo_type: Some(FloType::Source),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                     root,
                     context,
                     op_span,
                     ident,
                     arguments,
                     ..
                 },
                 _| {
        let receiver = &arguments[0];
        let stream_ident = wc.make_ident("stream");
        let write_prologue = quote_spanned! {op_span=>
            let mut #stream_ident = {
                #[inline(always)]
                fn check_stream<Stream: #root::futures::stream::Stream<Item = Item> + ::std::marker::Unpin, Item>(stream: Stream)
                    -> impl #root::futures::stream::Stream<Item = Item> + ::std::marker::Unpin
                {
                    stream
                }
                check_stream(#receiver)
            };
        };
        let fused_ident = wc.make_ident("stream_fused");
        let write_iterator = quote_spanned! {op_span=>
            let mut #fused_ident = #root::dfir_pipes::pull::Pull::fuse(#root::dfir_pipes::pull::stream_ready(
                &mut #stream_ident,
                #context.waker(),
            ));
            let #ident = &mut #fused_ident;
        };

        // Drain remaining items from the stream to ensure the waker is registered
        // (poll_next must return Pending at least once to register the waker for
        // future sends to wake this DFIR). The fused pull returns Ended immediately
        // if Pending was already hit during the pipeline.
        let write_iterator_after = quote_spanned! {op_span=>
            {
                use #root::dfir_pipes::pull::Pull;
                let mut drain = ::std::pin::pin!(#fused_ident);
                while let #root::dfir_pipes::pull::PullStep::Ready(_, _) = drain.as_mut().pull(&mut ()) {}
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
