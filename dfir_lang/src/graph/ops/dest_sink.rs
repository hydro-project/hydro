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
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   is_pull,
                   arguments,
                   ..
               },
               _| {
        assert!(!is_pull);

        let sink_arg = &arguments[0];
        let sink_ident = wc.make_ident("sink");
        let write_prologue = quote_spanned! {op_span=>
            let mut #sink_ident = #sink_arg;
        };
        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                fn sink_guard<Si, Item>(sink: Si) -> impl #root::dfir_pipes::push::Push<Item, ()>
                where
                    Si: #root::futures::sink::Sink<Item>,
                    Si::Error: ::std::error::Error + 'static,
                {
                    #root::pin_project_lite::pin_project! {
                        struct IgnoreBrokenPipe<Si> {
                            #[pin]
                            sink: Si,
                            failed: bool,
                        }
                    }

                    fn recover<E>(
                        result: ::std::result::Result<(), E>,
                        failed: &mut bool,
                    ) -> ::std::result::Result<(), ::std::convert::Infallible>
                    where
                        E: ::std::error::Error + 'static,
                    {
                        match result {
                            Ok(()) => Ok(()),
                            Err(err) => {
                                *failed = true;
                                #root::dfir_lang::handle_dest_sink_error(err);
                                Ok(())
                            }
                        }
                    }

                    fn recover_poll<E>(
                        poll: ::std::task::Poll<::std::result::Result<(), E>>,
                        failed: &mut bool,
                    ) -> ::std::task::Poll<::std::result::Result<(), ::std::convert::Infallible>>
                    where
                        E: ::std::error::Error + 'static,
                    {
                        match poll {
                            ::std::task::Poll::Pending => ::std::task::Poll::Pending,
                            ::std::task::Poll::Ready(result) => {
                                ::std::task::Poll::Ready(recover(result, failed))
                            }
                        }
                    }

                    impl<Si, Item> #root::futures::sink::Sink<Item> for IgnoreBrokenPipe<Si>
                    where
                        Si: #root::futures::sink::Sink<Item>,
                        Si::Error: ::std::error::Error + 'static,
                    {
                        type Error = ::std::convert::Infallible;

                        fn poll_ready(
                            self: ::std::pin::Pin<&mut Self>,
                            cx: &mut ::std::task::Context<'_>,
                        ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                            let mut this = self.project();
                            if *this.failed {
                                return ::std::task::Poll::Ready(Ok(()));
                            }
                            recover_poll(
                                <Si as #root::futures::sink::Sink<Item>>::poll_ready(
                                    this.sink.as_mut(),
                                    cx,
                                ),
                                this.failed,
                            )
                        }

                        fn start_send(
                            self: ::std::pin::Pin<&mut Self>,
                            item: Item,
                        ) -> ::std::result::Result<(), Self::Error> {
                            let mut this = self.project();
                            if *this.failed {
                                ::std::mem::drop(item);
                                return Ok(());
                            }
                            recover(
                                <Si as #root::futures::sink::Sink<Item>>::start_send(
                                    this.sink.as_mut(),
                                    item,
                                ),
                                this.failed,
                            )
                        }

                        fn poll_flush(
                            self: ::std::pin::Pin<&mut Self>,
                            cx: &mut ::std::task::Context<'_>,
                        ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                            let mut this = self.project();
                            if *this.failed {
                                return ::std::task::Poll::Ready(Ok(()));
                            }
                            recover_poll(
                                <Si as #root::futures::sink::Sink<Item>>::poll_flush(
                                    this.sink.as_mut(),
                                    cx,
                                ),
                                this.failed,
                            )
                        }

                        fn poll_close(
                            self: ::std::pin::Pin<&mut Self>,
                            cx: &mut ::std::task::Context<'_>,
                        ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                            let mut this = self.project();
                            if *this.failed {
                                return ::std::task::Poll::Ready(Ok(()));
                            }
                            recover_poll(
                                <Si as #root::futures::sink::Sink<Item>>::poll_close(
                                    this.sink.as_mut(),
                                    cx,
                                ),
                                this.failed,
                            )
                        }
                    }

                    #root::dfir_pipes::push::sink(IgnoreBrokenPipe {
                        sink,
                        failed: false,
                    })
                }
                sink_guard(&mut #sink_ident)
            };
        };
        let write_iterator_after = quote_spanned! {op_span=>};
        let write_tick_end = quote_spanned! {op_span=>};

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            write_tick_end,
        })
    },
};

/// Whether `err` or any [`std::error::Error::source`] is a broken pipe.
///
/// `'static` lets us downcast to [`std::io::Error`] instead of matching `Debug` text.
pub(crate) fn is_broken_pipe(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut next = Some(err);
    while let Some(err) = next {
        if err
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::BrokenPipe)
        {
            return true;
        }
        next = err.source();
    }
    false
}

/// Logs a broken pipe and panics on any other `dest_sink` error.
///
/// A broken pipe means the receiver closed first, which is normal during shutdown.
/// The failed send is dropped so the process can finish.
pub(crate) fn handle_dest_sink_error<E>(err: E)
where
    E: std::error::Error + 'static,
{
    if is_broken_pipe(&err) {
        tracing::debug!("dest_sink: broken pipe, dropping send: {err:?}");
    } else {
        panic!("{:?}", err);
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt::{self, Display};
    use std::io::ErrorKind;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    use super::is_broken_pipe;
    use crate::handle_dest_sink_error;

    fn io_error(kind: ErrorKind) -> std::io::Error {
        std::io::Error::new(kind, "dest_sink test")
    }

    /// `Display` + `Error` wrapper so the broken-pipe check has to walk `source`.
    #[derive(Debug)]
    struct Wrap(std::io::Error);

    impl Display for Wrap {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("wrapped io error")
        }
    }

    impl Error for Wrap {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            Some(&self.0)
        }
    }

    fn panic_message<E>(err: E) -> String
    where
        E: Error + 'static,
    {
        let expected = format!("{err:?}");
        let payload = catch_unwind(AssertUnwindSafe(|| handle_dest_sink_error(err)))
            .expect_err("sink error should panic");
        payload
            .downcast_ref::<String>()
            .unwrap_or_else(|| panic!("panic payload was not the Debug string {expected}"))
            .clone()
    }

    #[test]
    fn broken_pipe_is_quiet() {
        let err = io_error(ErrorKind::BrokenPipe);
        assert!(is_broken_pipe(&err));
        handle_dest_sink_error(err);
    }

    #[test]
    fn not_found_panics_with_debug_string() {
        let err = io_error(ErrorKind::NotFound);
        assert!(!is_broken_pipe(&err));
        let expected = format!("{err:?}");
        assert_eq!(expected, panic_message(err));
    }

    #[test]
    fn broken_pipe_source_is_quiet() {
        let err = Wrap(io_error(ErrorKind::BrokenPipe));
        assert!(is_broken_pipe(&err));
        handle_dest_sink_error(err);
    }

    #[test]
    fn not_found_source_panics_with_debug_string() {
        let err = Wrap(io_error(ErrorKind::NotFound));
        assert!(!is_broken_pipe(&err));
        let expected = format!("{err:?}");
        assert_eq!(expected, panic_message(err));
    }
}
