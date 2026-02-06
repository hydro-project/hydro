//! [`LazySinkSource`], and related items.

use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::Wake;

use futures_util::{Sink, Stream, ready};

struct MultiWaker {
    wakers: Mutex<Vec<Waker>>,
}

impl MultiWaker {
    fn new(waker: &Waker) -> Self {
        MultiWaker {
            wakers: Mutex::new(vec![waker.clone()]),
        }
    }

    fn push(&self, waker: &Waker) {
        let mut guard = self.wakers.lock().unwrap();
        guard.push(waker.clone());
    }
}

impl Wake for MultiWaker {
    fn wake(self: Arc<Self>) {
        let mut wakers = Vec::new();

        {
            let mut guard = self.wakers.lock().unwrap();
            std::mem::swap(&mut wakers, &mut *guard);
        }

        for waker in wakers {
            waker.wake();
        }
    }
}

enum SharedState<Fut, St, Si, Item> {
    Uninit {
        future: Pin<Box<Fut>>,
    },
    Thunkulating {
        future: Pin<Box<Fut>>,
        item: Option<Item>,
        multi_waker: Option<Arc<MultiWaker>>,
    },
    Done {
        stream: Pin<Box<St>>,
        sink: Pin<Box<Si>>,
        buf: Option<Item>,
    },
    Taken,
}

/// A lazy sink-source that can be split into a sink and a source. The internal state is initialized when the first item is attempted to be pulled from the source half, or when the first item is sent to the sink half.
pub struct LazySinkSource<Fut, St, Si, Item, Error> {
    state: Rc<RefCell<SharedState<Fut, St, Si, Item>>>,
    _phantom: PhantomData<Error>,
}

impl<Fut, St, Si, Item, Error> LazySinkSource<Fut, St, Si, Item, Error> {
    /// Creates a new `LazySinkSource` with the given initialization future.
    pub fn new(future: Fut) -> Self {
        Self {
            state: Rc::new(RefCell::new(SharedState::Uninit {
                future: Box::pin(future),
            })),
            _phantom: PhantomData,
        }
    }

    #[expect(
        clippy::type_complexity,
        reason = "this type is actually fine and not too complex."
    )]
    /// Splits into a sink and stream that share the same underlying connection.
    pub fn split(
        self,
    ) -> (
        LazySinkHalf<Fut, St, Si, Item, Error>,
        LazySourceHalf<Fut, St, Si, Item, Error>,
    ) {
        let sink = LazySinkHalf {
            state: Rc::clone(&self.state),
            _phantom: PhantomData,
        };
        let stream = LazySourceHalf {
            state: self.state,
            _phantom: PhantomData,
        };
        (sink, stream)
    }
}

/// Sink half of the SinkSource
pub struct LazySinkHalf<Fut, St, Si, Item, Error> {
    state: Rc<RefCell<SharedState<Fut, St, Si, Item>>>,
    _phantom: PhantomData<Error>,
}

/// Stream half of the SinkSource
pub struct LazySourceHalf<Fut, St, Si, Item, Error> {
    state: Rc<RefCell<SharedState<Fut, St, Si, Item>>>,
    _phantom: PhantomData<Error>,
}

impl<Fut, St, Si, Item, Error> Sink<Item> for LazySinkHalf<Fut, St, Si, Item, Error>
where
    Fut: Future<Output = Result<(St, Si), Error>>,
    St: Stream,
    Si: Sink<Item>,
    Error: From<Si::Error>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.state.borrow_mut();

        if let SharedState::Uninit { .. } = &*state {
            return Poll::Ready(Ok(()));
        }

        if let SharedState::Thunkulating {
            future,
            item,
            multi_waker,
        } = &mut *state
        {
            let waker = if let Some(waker) = multi_waker {
                waker.push(cx.waker());
                Waker::from(waker.clone())
            } else {
                let waker = Arc::new(MultiWaker::new(cx.waker()));
                *multi_waker = Some(waker.clone());
                Waker::from(waker)
            };

            let mut new_context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut new_context) {
                Poll::Ready(Ok((stream, sink))) => {
                    let buf = item.take();
                    *state = SharedState::Done {
                        stream: Box::pin(stream),
                        sink: Box::pin(sink),
                        buf,
                    };
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        if let SharedState::Done { sink, buf, .. } = &mut *state {
            if buf.is_some() {
                ready!(sink.as_mut().poll_ready(cx).map_err(From::from)?);
                sink.as_mut().start_send(buf.take().unwrap())?;
            }
            let result = sink.as_mut().poll_ready(cx).map_err(From::from);
            return result;
        }

        panic!("LazySinkHalf in invalid state.");
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let mut state = self.state.borrow_mut();

        if let SharedState::Uninit { .. } = &*state {
            let old_state = std::mem::replace(&mut *state, SharedState::Taken);
            if let SharedState::Uninit { future } = old_state {
                *state = SharedState::Thunkulating {
                    future,
                    item: Some(item),
                    multi_waker: None,
                };

                return Ok(());
            }
        }

        if let SharedState::Thunkulating { .. } = &mut *state {
            panic!("LazySinkHalf not ready.");
        }

        if let SharedState::Done { sink, buf, .. } = &mut *state {
            debug_assert!(buf.is_none());
            let result = sink.as_mut().start_send(item).map_err(From::from);
            return result;
        }

        panic!("LazySinkHalf not ready.");
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.state.borrow_mut();

        if let SharedState::Uninit { .. } = &*state {
            return Poll::Ready(Ok(()));
        }

        if let SharedState::Thunkulating {
            future,
            item,
            multi_waker,
        } = &mut *state
        {
            let waker = if let Some(waker) = multi_waker {
                waker.push(cx.waker());
                Waker::from(waker.clone())
            } else {
                let waker = Arc::new(MultiWaker::new(cx.waker()));
                *multi_waker = Some(waker.clone());
                Waker::from(waker)
            };

            let mut new_context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut new_context) {
                Poll::Ready(Ok((stream, sink))) => {
                    let buf = item.take();
                    *state = SharedState::Done {
                        stream: Box::pin(stream),
                        sink: Box::pin(sink),
                        buf,
                    };
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        if let SharedState::Done { sink, buf, .. } = &mut *state {
            if buf.is_some() {
                ready!(sink.as_mut().poll_ready(cx).map_err(From::from)?);
                sink.as_mut().start_send(buf.take().unwrap())?;
            }
            let result = sink.as_mut().poll_flush(cx).map_err(From::from);
            return result;
        }

        panic!("LazySinkHalf in invalid state.");
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut state = self.state.borrow_mut();

        if let SharedState::Uninit { .. } = &*state {
            return Poll::Ready(Ok(()));
        }

        if let SharedState::Thunkulating {
            future,
            item,
            multi_waker,
        } = &mut *state
        {
            let waker = if let Some(waker) = multi_waker {
                waker.push(cx.waker());
                Waker::from(waker.clone())
            } else {
                let waker = Arc::new(MultiWaker::new(cx.waker()));
                *multi_waker = Some(waker.clone());
                Waker::from(waker)
            };

            let mut new_context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut new_context) {
                Poll::Ready(Ok((stream, sink))) => {
                    let buf = item.take();
                    *state = SharedState::Done {
                        stream: Box::pin(stream),
                        sink: Box::pin(sink),
                        buf,
                    };
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        if let SharedState::Done { sink, buf, .. } = &mut *state {
            if buf.is_some() {
                ready!(sink.as_mut().poll_ready(cx).map_err(From::from)?);
                sink.as_mut().start_send(buf.take().unwrap())?;
            }
            let result = sink.as_mut().poll_close(cx).map_err(From::from);
            return result;
        }

        panic!("LazySinkHalf in invalid state.");
    }
}

impl<Fut, St, Si, Item, Error> Stream for LazySourceHalf<Fut, St, Si, Item, Error>
where
    Fut: Future<Output = Result<(St, Si), Error>>,
    St: Stream,
    Si: Sink<Item>,
{
    type Item = St::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut state = self.state.borrow_mut();

        if let SharedState::Uninit { .. } = &*state {
            let old_state = std::mem::replace(&mut *state, SharedState::Taken);
            if let SharedState::Uninit { future } = old_state {
                *state = SharedState::Thunkulating {
                    future,
                    item: None,
                    multi_waker: None,
                };
            } else {
                unreachable!();
            }
        }

        if let SharedState::Thunkulating {
            future,
            item,
            multi_waker,
        } = &mut *state
        {
            let waker = if let Some(waker) = multi_waker {
                waker.push(cx.waker());
                Waker::from(waker.clone())
            } else {
                let waker = Arc::new(MultiWaker::new(cx.waker()));
                *multi_waker = Some(waker.clone());
                Waker::from(waker)
            };

            let mut new_context = Context::from_waker(&waker);

            match future.as_mut().poll(&mut new_context) {
                Poll::Ready(Ok((stream, sink))) => {
                    let buf = item.take();
                    *state = SharedState::Done {
                        stream: Box::pin(stream),
                        sink: Box::pin(sink),
                        buf,
                    };
                }

                Poll::Ready(Err(_)) => {
                    return Poll::Ready(None);
                }

                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        if let SharedState::Done { stream, .. } = &mut *state {
            let result = stream.as_mut().poll_next(cx);
            match &result {
                Poll::Ready(Some(_)) => {}
                Poll::Ready(None) => {}
                Poll::Pending => {}
            }
            return result;
        }

        panic!("LazySourceHalf in invalid state.");
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

                // Ensures stream starts the lazy.
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

                assert!(!sink_task.is_finished()); // We haven't sent anything yet, so the stream should definitely not be resolved now.

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
