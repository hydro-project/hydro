use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// > Arguments: An [async `Sink`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html).
///
/// Consumes items by sending them to an [async `Sink`](https://docs.rs/futures/latest/futures/sink/trait.Sink.html).
/// A `Sink` is a thing into which values can be sent, asynchronously. For example, sending items
/// into a bounded channel.
///
/// Note this operator must be used within a Tokio runtime, and the DFIR program must be launched with `run_async`.
///
/// ```rustbook
/// # #[dfir_rs::main]
/// # async fn main() {
/// // In this example we use a _bounded_ channel for our `Sink`. This is for demonstration only,
/// // instead you should use [`dfir_rs::util::unbounded_channel`]. A bounded channel results in
/// // buffering items internally instead of within the channel. (We can't use
/// // unbounded here since unbounded channels are synchonous to write to and therefore not
/// // `Sink`s.)
/// let (send, recv) = tokio::sync::mpsc::channel::<usize>(5);
/// // `PollSender` adapts the send half of the bounded channel into a `Sink`.
/// let send = tokio_util::sync::PollSender::new(send);
///
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_iter(0..10) -> dest_sink(send);
/// };
/// // Call `run_async()` to allow async events to propagate, run for one second.
/// tokio::time::timeout(std::time::Duration::from_secs(1), flow.run())
///     .await
///     .expect_err("Expected time out");
///
/// let mut recv = tokio_stream::wrappers::ReceiverStream::new(recv);
/// // Only 5 elements received due to buffer size.
/// // (Note that if we were using a multi-threaded executor instead of `current_thread` it would
/// // be possible for more items to be added as they're removed, resulting in >5 collected.)
/// let out: Vec<_> = dfir_rs::util::ready_iter(&mut recv).collect();
/// assert_eq!(&[0, 1, 2, 3, 4], &*out);
/// # }
/// ```
///
/// `Sink` is different from [`AsyncWrite`](https://docs.rs/futures/latest/futures/io/trait.AsyncWrite.html).
/// Instead of discrete values we send arbitrary streams of bytes into an `AsyncWrite` value. For
/// example, writings a stream of bytes to a file, a socket, or stdout.
///
/// To handle those situations we can use a codec from [`tokio_util::codec`](https://docs.rs/tokio-util/latest/tokio_util/codec/index.html).
/// These specify ways in which the byte stream is broken into individual items, such as with
/// newlines or with length delineation.
///
/// If we only want to write a stream of bytes without delineation we can use the [`BytesCodec`](https://docs.rs/tokio-util/latest/tokio_util/codec/struct.BytesCodec.html).
///
/// In this example we use a [`duplex`](https://docs.rs/tokio/latest/tokio/io/fn.duplex.html) as our `AsyncWrite` with a
/// `BytesCodec`.
///
/// ```rustbook
/// # #[dfir_rs::main]
/// # async fn main() {
/// use bytes::Bytes;
/// use tokio::io::AsyncReadExt;
///
/// // Like a channel, but for a stream of bytes instead of discrete objects.
/// let (asyncwrite, mut asyncread) = tokio::io::duplex(256);
/// // Now instead handle discrete byte strings by length-encoding them.
/// let sink = tokio_util::codec::FramedWrite::new(asyncwrite, tokio_util::codec::BytesCodec::new());
///
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_iter([
///         Bytes::from_static(b"hello"),
///         Bytes::from_static(b"world"),
///     ]) -> dest_sink(sink);
/// };
/// tokio::time::timeout(std::time::Duration::from_secs(1), flow.run())
///     .await
///     .expect_err("Expected time out");
///
/// let mut buf = Vec::<u8>::new();
/// asyncread.read_buf(&mut buf).await.unwrap();
/// assert_eq!(b"helloworld", &*buf);
/// # }
/// ```
pub const DEST_SINK: OperatorConstraints = OperatorConstraints {
    name: "dest_sink",
    categories: &[OperatorCategory::Sink],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_0,
    soft_range_out: RANGE_0,
    num_args: 1,
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
                   df_ident,
                   op_span,
                   ident,
                   is_pull,
                   arguments,
                   work_fn,
                   ..
               },
               _| {
        assert!(!is_pull);

        let sink_arg = &arguments[0];

        let send_ident = wc.make_ident("item_send");
        let recv_ident = wc.make_ident("item_recv");

        let write_prologue = quote_spanned! {op_span=>
            let (#send_ident, #recv_ident) = #root::tokio::sync::mpsc::unbounded_channel();
            {
                /// Function is needed so `Item` is so no ambiguity for what `Item` is used
                /// when calling `.flush()`.
                #[allow(non_snake_case)]
                async fn #work_fn<Sink, Item>(
                    mut recv: #root::tokio::sync::mpsc::UnboundedReceiver<Item>,
                    mut sink: Sink,
                ) where
                    Sink: ::std::marker::Unpin + #root::futures::Sink<Item>,
                    Sink::Error: ::std::fmt::Debug,
                {
                    use #root::futures::SinkExt;
                    while let Some(item) = recv.recv().await {
                        sink.feed(item)
                            .await
                            .expect("Error processing async sink item");
                        while let Ok(item) = recv.try_recv() {
                            sink.feed(item)
                                .await
                                .expect("Error processing async sink item");
                        }
                        sink.flush().await.expect("Failed to flush sink");
                    }
                }
                #df_ident
                    .request_task(#work_fn(#recv_ident, #sink_arg));
            }
        };

        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::pusherator::for_each::ForEach::new(|item| {
                if let Err(err) = #send_ident.send(item) {
                    panic!("Failed to send async write item for processing: {}", err);
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
