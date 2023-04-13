use super::{make_missing_runtime_msg, FlowProperties, FlowPropertyVal};

use super::{
    OperatorConstraints, OperatorInstance, OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};

use quote::quote_spanned;

/// The same as `dest_sink`, but takes two additional parameters controlling
/// when the data is actually flushed.
#[hydroflow_internalmacro::operator_docgen]
pub const DEST_SINK_CHUNKED: OperatorConstraints = OperatorConstraints {
    name: "dest_sink_chunked",
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_0,
    soft_range_out: RANGE_0,
    num_args: 3,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    ports_inn: None,
    ports_out: None,
    properties: FlowProperties {
        deterministic: FlowPropertyVal::Preserve,
        monotonic: FlowPropertyVal::Preserve,
        inconsistency_tainted: false,
    },
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   hydroflow,
                   op_span,
                   ident,
                   op_name,
                   op_inst: OperatorInstance { arguments, .. },
                   ..
               },
               _| {
        let sink_arg = &arguments[0];
        let chunk_size_arg = &arguments[1];
        let chunk_delay_arg = &arguments[2];

        let send_ident = wc.make_ident("item_send");
        let recv_ident = wc.make_ident("item_recv");

        let missing_runtime_msg = make_missing_runtime_msg(op_name);

        let write_prologue = quote_spanned! {op_span=>
            let (#send_ident, #recv_ident) = #root::tokio::sync::mpsc::unbounded_channel();
            {
                /// Function is needed so `Item` is so no ambiguity for what `Item` is used
                /// when calling `.flush()`.
                async fn sink_feed_flush<Sink, Item>(
                    recv: #root::tokio::sync::mpsc::UnboundedReceiver<Item>,
                    mut sink: Sink,
                    chunk_size: usize,
                    delay: ::std::time::Duration,
                ) where
                    Sink: ::std::marker::Unpin + #root::futures::Sink<Item>,
                    Sink::Error: ::std::fmt::Debug,
                {
                    use #root::futures::SinkExt;
                    use #root::futures::StreamExt;
                    use #root::futures_batch::ChunksTimeoutStreamExt;

                    let recv_stream = #root::tokio_stream::wrappers::UnboundedReceiverStream::new(recv);
                    let mut batched_recv = ::std::boxed::Box::pin(recv_stream.chunks_timeout(chunk_size, delay));

                    while let Some(batch) = batched_recv.next().await {
                        for item in batch {
                            sink.feed(item)
                                .await
                                .expect("Error processing async sink item.");
                        }

                        sink.flush().await.expect("Failed to flush sink.");
                    }
                }
                #hydroflow
                    .spawn_task(sink_feed_flush(#recv_ident, #sink_arg, #chunk_size_arg, #chunk_delay_arg))
                    .expect(#missing_runtime_msg);
            }
        };

        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::pusherator::for_each::ForEach::new(|item| {
                #send_ident.send(item).expect("Failed to send async write item for processing.");
            });
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
