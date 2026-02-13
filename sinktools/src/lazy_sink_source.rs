//! [`LazySinkSource`], and related items.

use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::sync::Arc;
use std::task::Wake;

use futures_util::task::AtomicWaker;
use futures_util::{Sink, Stream, ready};
use pin_project_lite::pin_project;

#[derive(Default)]
struct DualWaker {
    sink: AtomicWaker,
    stream: AtomicWaker,
}

impl DualWaker {
    fn new() -> (Arc<Self>, Waker) {
        let dual_waker = Arc::new(Self::default());
        let waker = Waker::from(dual_waker.clone());
        (dual_waker, waker)
    }
}

impl Wake for DualWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.sink.wake();
        self.stream.wake();
    }
}

pin_project! {
    #[project = SharedStateProj]
    enum SharedState<Fut, St, Si, Item> {
        Uninit {
            // The future, always `Some` in this state.
            future: Option<Fut>,
        },
        Thunkulating {
            #[pin]
            future: Fut,
            item: Option<Item>,
            dual_waker_state: Arc<DualWaker>,
            dual_waker_waker: Waker,
        },
        Done {
            #[pin]
            stream: St,
            #[pin]
            sink: Si,
            buf: Option<Item>,
        },
    }
}

pin_project! {
    /// A lazy sink-source, where the internal state is initialized when the first item is attempted to be pulled from the
    /// source, or when the first item is sent to the sink. To split into separate source and sink halves, use
    /// [`futures_util::StreamExt::split`].
    pub struct LazySinkSource<Fut, St, Si, Item, Error> {
        #[pin]
        state: SharedState<Fut, St, Si, Item>,
        _phantom: PhantomData<Error>,
    }
}

impl<Fut, St, Si, Item, Error> LazySinkSource<Fut, St, Si, Item, Error> {
    /// Creates a new `LazySinkSource` with the given initialization future.
    pub fn new(future: Fut) -> Self {
        Self {
            state: SharedState::Uninit {
                future: Some(future),
            },
            _phantom: PhantomData,
        }
    }
}

impl<Fut, St, Si, Item, Error> LazySinkSource<Fut, St, Si, Item, Error>
where
    Fut: Future<Output = Result<(St, Si), Error>>,
    St: Stream,
    Si: Sink<Item>,
    Error: From<Si::Error>,
{
    fn poll_sink_op(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        sink_op: impl FnOnce(Pin<&mut Si>, &mut Context<'_>) -> Poll<Result<(), Si::Error>>,
    ) -> Poll<Result<(), Error>> {
        let mut this = self.project();

        if let SharedStateProj::Uninit { .. } = this.state.as_mut().project() {
            return Poll::Ready(Ok(()));
        }

        if let SharedStateProj::Thunkulating {
            future,
            item,
            dual_waker_state,
            dual_waker_waker,
        } = this.state.as_mut().project()
        {
            dual_waker_state.sink.register(cx.waker());

            let mut dual_context = Context::from_waker(dual_waker_waker);

            match future.poll(&mut dual_context) {
                Poll::Ready(Ok((stream, sink))) => {
                    let buf = item.take();
                    this.state
                        .as_mut()
                        .set(SharedState::Done { stream, sink, buf });
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        if let SharedStateProj::Done { mut sink, buf, .. } = this.state.as_mut().project() {
            if buf.is_some() {
                ready!(sink.as_mut().poll_ready(cx).map_err(From::from)?);
                sink.as_mut().start_send(buf.take().unwrap())?;
            }
            return (sink_op)(sink, cx).map_err(From::from);
        }

        panic!("LazySinkSource in invalid state.");
    }
}

impl<Fut, St, Si, Item, Error> Sink<Item> for LazySinkSource<Fut, St, Si, Item, Error>
where
    Fut: Future<Output = Result<(St, Si), Error>>,
    St: Stream,
    Si: Sink<Item>,
    Error: From<Si::Error>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sink_op(cx, Sink::poll_ready)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let mut this = self.project();

        if let SharedStateProj::Uninit { future } = this.state.as_mut().project() {
            let future = future.take().unwrap();
            let (dual_waker_state, dual_waker_waker) = DualWaker::new();
            this.state.as_mut().set(SharedState::Thunkulating {
                future,
                item: Some(item),
                dual_waker_state,
                dual_waker_waker,
            });
            return Ok(());
        }

        if let SharedStateProj::Thunkulating { .. } = this.state.as_mut().project() {
            panic!("LazySinkSource not ready.");
        }

        if let SharedStateProj::Done { sink, buf, .. } = this.state.as_mut().project() {
            debug_assert!(buf.is_none());
            return sink.start_send(item).map_err(From::from);
        }

        panic!("LazySinkSource not ready.");
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sink_op(cx, Sink::poll_flush)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sink_op(cx, Sink::poll_close)
    }
}

impl<Fut, St, Si, Item, Error> Stream for LazySinkSource<Fut, St, Si, Item, Error>
where
    Fut: Future<Output = Result<(St, Si), Error>>,
    St: Stream,
    Si: Sink<Item>,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if let SharedStateProj::Uninit { future } = this.state.as_mut().project() {
            let future = future.take().unwrap();
            let (dual_waker_state, dual_waker_waker) = DualWaker::new();
            this.state.as_mut().set(SharedState::Thunkulating {
                future,
                item: None,
                dual_waker_state,
                dual_waker_waker,
            });
        }

        if let SharedStateProj::Thunkulating {
            future,
            item,
            dual_waker_state,
            dual_waker_waker,
        } = this.state.as_mut().project()
        {
            dual_waker_state.stream.register(cx.waker());

            let mut new_context = Context::from_waker(dual_waker_waker);

            match future.poll(&mut new_context) {
                Poll::Ready(Ok((stream, sink))) => {
                    let buf = item.take();
                    this.state
                        .as_mut()
                        .set(SharedState::Done { stream, sink, buf });
                }

                Poll::Ready(Err(_)) => {
                    return Poll::Ready(None);
                }

                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        if let SharedStateProj::Done { stream, .. } = this.state.as_mut().project() {
            return stream.poll_next(cx);
        }

        panic!("LazySinkSource in invalid state.");
    }
}

#[cfg(test)]
mod test {
    use futures_util::{SinkExt, StreamExt};
    use tokio_util::sync::PollSendError;

    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn stream_drives_initialization() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (init_lazy_send, init_lazy_recv) = tokio::sync::oneshot::channel::<()>();

                let sink_source = LazySinkSource::new(async move {
                    let () = init_lazy_recv.await.unwrap();
                    let (send, recv) = tokio::sync::mpsc::channel(1);
                    let sink = tokio_util::sync::PollSender::new(send);
                    let stream = tokio_stream::wrappers::ReceiverStream::new(recv);
                    Ok::<_, PollSendError<_>>((stream, sink))
                });

                let (mut sink, mut stream) = sink_source.split();

                // Ensures stream starts the lazy.
                let (stream_init_send, stream_init_recv) = tokio::sync::oneshot::channel::<()>();
                let stream_task = tokio::task::spawn_local(async move {
                    stream_init_send.send(()).unwrap();
                    (stream.next().await.unwrap(), stream.next().await.unwrap())
                });
                let sink_task = tokio::task::spawn_local(async move {
                    stream_init_recv.await.unwrap();
                    SinkExt::send(&mut sink, "test1").await.unwrap();
                    SinkExt::send(&mut sink, "test2").await.unwrap();
                });

                // finish the future.
                init_lazy_send.send(()).unwrap();

                tokio::task::yield_now().await;

                assert!(sink_task.is_finished());
                assert_eq!(("test1", "test2"), stream_task.await.unwrap());
                sink_task.await.unwrap();
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sink_drives_initialization() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (init_lazy_send, init_lazy_recv) = tokio::sync::oneshot::channel::<()>();

                let sink_source = LazySinkSource::new(async move {
                    let () = init_lazy_recv.await.unwrap();
                    let (send, recv) = tokio::sync::mpsc::channel(1);
                    let sink = tokio_util::sync::PollSender::new(send);
                    let stream = tokio_stream::wrappers::ReceiverStream::new(recv);
                    Ok::<_, PollSendError<_>>((stream, sink))
                });

                let (mut sink, mut stream) = sink_source.split();

                // Ensures sink starts the lazy.
                let (sink_init_send, sink_init_recv) = tokio::sync::oneshot::channel::<()>();
                let stream_task = tokio::task::spawn_local(async move {
                    sink_init_recv.await.unwrap();
                    (stream.next().await.unwrap(), stream.next().await.unwrap())
                });
                let sink_task = tokio::task::spawn_local(async move {
                    sink_init_send.send(()).unwrap();
                    SinkExt::send(&mut sink, "test1").await.unwrap();
                    SinkExt::send(&mut sink, "test2").await.unwrap();
                });

                // finish the future.
                init_lazy_send.send(()).unwrap();

                tokio::task::yield_now().await;

                assert!(sink_task.is_finished());
                assert_eq!(("test1", "test2"), stream_task.await.unwrap());
                sink_task.await.unwrap();
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_stream_drives_initialization() {
        use tokio::net::{TcpListener, TcpStream};
        use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

        let (initialization_tx, initialization_rx) = tokio::sync::oneshot::channel::<()>();

        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();

                let sink_source = LazySinkSource::new(async move {
                    // initialization is at least partially started now.
                    initialization_tx.send(()).unwrap();

                    let (stream, _) = listener.accept().await.unwrap();
                    let (rx, tx) = stream.into_split();
                    let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
                    let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());
                    Ok::<_, std::io::Error>((fr, fw))
                });

                let (mut sink, mut stream) = sink_source.split();

                let stream_task = tokio::task::spawn_local(async move { stream.next().await });

                initialization_rx.await.unwrap(); // ensure that the runtime starts driving initialization via the stream.next() call.

                let sink_task = tokio::task::spawn_local(async move {
                    SinkExt::send(&mut sink, bytes::Bytes::from("test2"))
                        .await
                        .unwrap();
                });

                // try to be really sure that the above sink_task is waiting on the same future to be resolved.
                for _ in 0..20 {
                    tokio::task::yield_now().await
                }

                // trigger further initialization of the future.
                let mut socket = TcpStream::connect(addr).await.unwrap();
                let (client_rx, client_tx) = socket.split();
                let mut client_tx = FramedWrite::new(client_tx, LengthDelimitedCodec::new());
                let mut client_rx = FramedRead::new(client_rx, LengthDelimitedCodec::new());

                // try to be really sure that the effects of the above initialization completing are propagated.
                for _ in 0..20 {
                    tokio::task::yield_now().await
                }

                assert!(!stream_task.is_finished()); // We haven't sent anything yet, so the stream should definitely not be resolved now.

                // Now actually send an item so that the stream will wake up and have an item ready to pull from it.
                SinkExt::send(&mut client_tx, bytes::Bytes::from("test"))
                    .await
                    .unwrap();

                assert_eq!(&stream_task.await.unwrap().unwrap().unwrap()[..], b"test");
                sink_task.await.unwrap();

                assert_eq!(&client_rx.next().await.unwrap().unwrap()[..], b"test2");
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_sink_drives_initialization() {
        use tokio::net::{TcpListener, TcpStream};
        use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

        let (initialization_tx, initialization_rx) = tokio::sync::oneshot::channel::<()>();

        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();

                let sink_source = LazySinkSource::new(async move {
                    // initialization is at least partially started now.
                    initialization_tx.send(()).unwrap();

                    let (stream, _) = listener.accept().await.unwrap();
                    let (rx, tx) = stream.into_split();
                    let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
                    let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());
                    Ok::<_, std::io::Error>((fr, fw))
                });

                let (mut sink, mut stream) = sink_source.split();

                let sink_task = tokio::task::spawn_local(async move {
                    SinkExt::send(&mut sink, bytes::Bytes::from("test2"))
                        .await
                        .unwrap();
                });

                initialization_rx.await.unwrap(); // ensure that the runtime starts driving initialization via the stream.next() call.

                let stream_task = tokio::task::spawn_local(async move { stream.next().await });

                // try to be really sure that the above sink_task is waiting on the same future to be resolved.
                for _ in 0..20 {
                    tokio::task::yield_now().await
                }

                assert!(!sink_task.is_finished(), "We haven't sent anything yet, so the sink should definitely not be resolved now.");

                // trigger further initialization of the future.
                let mut socket = TcpStream::connect(addr).await.unwrap();
                let (client_rx, client_tx) = socket.split();
                let mut client_tx = FramedWrite::new(client_tx, LengthDelimitedCodec::new());
                let mut client_rx = FramedRead::new(client_rx, LengthDelimitedCodec::new());

                // try to be really sure that the effects of the above initialization completing are propagated.
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;

                assert!(sink_task.is_finished()); // Sink should have sent its item.

                assert_eq!(&client_rx.next().await.unwrap().unwrap()[..], b"test2");

                // Now actually send an item so that the stream will wake up and have an item ready to pull from it.
                SinkExt::send(&mut client_tx, bytes::Bytes::from("test"))
                    .await
                    .unwrap();

                assert_eq!(&stream_task.await.unwrap().unwrap().unwrap()[..], b"test");
                sink_task.await.unwrap();
            })
            .await;
    }
}
