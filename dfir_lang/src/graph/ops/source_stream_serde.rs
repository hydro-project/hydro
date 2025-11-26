use quote::quote_spanned;

use crate::graph::{OpInstGenerics, OperatorInstance};

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)
///
/// Given a [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)
/// of `(serialized payload, addr)` pairs, deserializes the payload and emits each of the
/// elements it receives downstream.
///
/// ```rustbook
/// async fn serde_in() {
///     let addr = dfir_rs::util::ipv4_resolve("localhost:9000".into()).unwrap();
///     let (outbound, inbound, _) = dfir_rs::util::bind_udp_bytes(addr).await;
///     let mut flow = dfir_rs::dfir_syntax! {
///         source_stream_serde(inbound) -> map(Result::unwrap) -> map(|(x, a): (String, std::net::SocketAddr)| x.to_uppercase())
///             -> for_each(|x| println!("{}", x));
///     };
///     flow.run_available();
/// }
/// ```
pub const SOURCE_STREAM_SERDE: OperatorConstraints = OperatorConstraints {
    name: "source_stream_serde",
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
                   context,
                   op_span,
                   ident,
                   arguments,
                   op_inst:
                       OperatorInstance {
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               _| {
        let generic_type = type_args
            .first()
            .map(quote::ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let receiver = &arguments[0];
        let stream_ident = wc.make_ident("stream");
        let write_prologue = quote_spanned! {op_span=>
            // TODO(mingwei): use `::std::pin::pin!(..)`?
            let mut #stream_ident = Box::pin(#receiver);
        };
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::futures::stream::poll_fn(|_tick_cx| {
                // Using the `tick_cx` will cause the tick to "block" (yield) until the stream is exhausted, which is not what we want.
                // We want only the ready items, and will awaken this subgraph on a later tick when more items are available.
                match #root::futures::stream::Stream::poll_next(#stream_ident.as_mut(), &mut ::std::task::Context::from_waker(&#context.waker())) {
                    ::std::task::Poll::Ready(Some(::std::result::Result::Ok((payload, addr)))) =>
                        ::std::task::Poll::Ready(::std::option::Option::Some(
                            #root::util::deserialize_from_bytes::<#generic_type>(payload).map(|payload| (payload, addr))
                        )),
                    ::std::task::Poll::Ready(Some(Err(_)))
                        | ::std::task::Poll::Ready(None)
                        | ::std::task::Poll::Pending => ::std::task::Poll::Ready(::std::option::Option::None),
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
