//! [`LazySinkSource`], and related items.

use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::cell::RefCell;
use std::rc::Rc;

use futures_util::{Sink, Stream, ready};

enum SharedState<Fut, St, Si, Item> {
    Uninit {
        future: Pin<Box<Fut>>,
    },
    Thunkulating {
        future: Pin<Box<Fut>>,
        item: Option<Item>,
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

        if let SharedState::Thunkulating { future, item } = &mut *state {
            match future.as_mut().poll(cx) {
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
                };

                return Ok(());
            }
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

        if let SharedState::Thunkulating { future, item } = &mut *state {
            match future.as_mut().poll(cx) {
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

        if let SharedState::Thunkulating { future, item } = &mut *state {
            match future.as_mut().poll(cx) {
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
                *state = SharedState::Thunkulating { future, item: None };
            }
        }

        if let SharedState::Thunkulating { future, item } = &mut *state {
            match future.as_mut().poll(cx) {
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
    use core::cell::RefCell;

    use futures_util::{SinkExt, StreamExt};

    use super::*;

    #[tokio::test]
    async fn test_lazy_sink_source() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let sink_source = LazySinkSource::new(async move {
            let stream = futures_util::stream::unfold(rx, |mut rx| async move {
                rx.recv().await.map(|item| (item, rx))
            });
            let sink = futures_util::sink::unfold(tx, |tx, item| async move {
                tx.send(item).ok();
                Ok::<_, ()>(tx)
            });
            Ok::<_, ()>((stream, sink))
        });

        let (mut sink, mut stream) = sink_source.split();

        SinkExt::send(&mut sink, 42).await.unwrap();
        assert_eq!(stream.next().await, Some(42));

        SinkExt::send(&mut sink, 100).await.unwrap();
        assert_eq!(stream.next().await, Some(100));
    }

    #[tokio::test]
    async fn test_lazy_sink_source_stream_first() {
        let init_count = Rc::new(RefCell::new(0));
        let init_count_clone = Rc::clone(&init_count);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let sink_source = LazySinkSource::new(async move {
            *init_count_clone.borrow_mut() += 1;
            let stream = futures_util::stream::unfold(rx, |mut rx| async move {
                rx.recv().await.map(|item| (item, rx))
            });
            let sink = futures_util::sink::unfold(tx, |tx, item| async move {
                tx.send(item).ok();
                Ok::<_, ()>(tx)
            });
            Ok::<_, ()>((stream, sink))
        });

        let (mut sink, mut stream) = sink_source.split();

        // Use stream first to trigger initialization and receive data
        SinkExt::send(&mut sink, 42).await.unwrap();
        assert_eq!(stream.next().await, Some(42));

        // Verify init was called exactly once
        assert_eq!(*init_count.borrow(), 1);
    }

    #[tokio::test]
    async fn test_lazy_sink_source_sink_first() {
        let init_count = Rc::new(RefCell::new(0));
        let init_count_clone = Rc::clone(&init_count);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let sink_source = LazySinkSource::new(async move {
            *init_count_clone.borrow_mut() += 1;
            let stream = futures_util::stream::unfold(rx, |mut rx| async move {
                rx.recv().await.map(|item| (item, rx))
            });
            let sink = futures_util::sink::unfold(tx, |tx, item| async move {
                tx.send(item).ok();
                Ok::<_, ()>(tx)
            });
            Ok::<_, ()>((stream, sink))
        });

        let (mut sink, mut stream) = sink_source.split();

        // Use sink first to trigger initialization
        SinkExt::send(&mut sink, 42).await.unwrap();

        // Verify init was called exactly once
        assert_eq!(*init_count.borrow(), 1);

        // Now use stream
        assert_eq!(stream.next().await, Some(42));

        // Verify init was still only called once
        assert_eq!(*init_count.borrow(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_lazy_sink_source_concurrent_stream_first() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

                let sink_source = LazySinkSource::new(async move {
                    ready_rx.await.ok();
                    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
                        rx.recv().await.map(|item| (item, rx))
                    });
                    let sink = futures_util::sink::unfold(tx, |tx, item| async move {
                        tx.send(item).ok();
                        Ok::<_, ()>(tx)
                    });
                    Ok::<_, ()>((stream, sink))
                });

                let (sink, stream) = sink_source.split();

                let stream_task = tokio::task::spawn_local(async move {
                    let mut stream = stream;
                    stream.next().await
                });

                let sink_task = tokio::task::spawn_local(async move {
                    let mut sink = sink;
                    tokio::task::yield_now().await;
                    ready_tx.send(()).ok();
                    SinkExt::send(&mut sink, 42).await.unwrap();
                });

                let result = stream_task.await.unwrap();
                sink_task.await.unwrap();

                assert_eq!(result, Some(42));
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_lazy_sink_source_concurrent_sink_first() {
        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

                let sink_source = LazySinkSource::new(async move {
                    ready_rx.await.ok();
                    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
                        rx.recv().await.map(|item| (item, rx))
                    });
                    let sink = futures_util::sink::unfold(tx, |tx, item| async move {
                        tx.send(item).ok();
                        Ok::<_, ()>(tx)
                    });
                    Ok::<_, ()>((stream, sink))
                });

                let (sink, stream) = sink_source.split();

                let sink_task = tokio::task::spawn_local(async move {
                    let mut sink = sink;
                    ready_tx.send(()).ok();
                    SinkExt::send(&mut sink, 42).await.unwrap();
                });

                let stream_task = tokio::task::spawn_local(async move {
                    let mut stream = stream;
                    tokio::task::yield_now().await;
                    stream.next().await
                });

                sink_task.await.unwrap();
                let result = stream_task.await.unwrap();

                assert_eq!(result, Some(42));
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_lazy_sink_source_tcp_stream_first() {
        use tokio::net::{TcpListener, TcpStream};
        use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();

                let sink_source = LazySinkSource::new(async move {
                    let (stream, _) = listener.accept().await.unwrap();
                    let (rx, tx) = stream.into_split();
                    let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
                    let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());
                    Ok::<_, std::io::Error>((fr, fw))
                });

                let (_sink, stream) = sink_source.split();

                let stream_task = tokio::task::spawn_local(async move {
                    let mut stream = stream;
                    stream.next().await
                });

                let client = TcpStream::connect(addr).await.unwrap();
                let mut client_sink = FramedWrite::new(client, LengthDelimitedCodec::new());
                SinkExt::send(&mut client_sink, bytes::Bytes::from("test"))
                    .await
                    .unwrap();

                let result = stream_task.await.unwrap();

                let bytes = result.unwrap().unwrap();
                assert_eq!(&bytes[..], b"test");
            })
            .await;
    }

    #[tokio::test(flavor = "current_thread")]
    async fn test_lazy_sink_source_tcp_sink_first() {
        use tokio::net::{TcpListener, TcpStream};
        use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

        let local = tokio::task::LocalSet::new();
        local
            .run_until(async {
                let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
                let addr = listener.local_addr().unwrap();

                let sink_source = LazySinkSource::new(async move {
                    let (stream, _) = listener.accept().await.unwrap();
                    let (rx, tx) = stream.into_split();
                    let fr = FramedRead::new(rx, LengthDelimitedCodec::new());
                    let fw = FramedWrite::new(tx, LengthDelimitedCodec::new());
                    Ok::<_, std::io::Error>((fr, fw))
                });

                let (sink, stream) = sink_source.split();

                let sink_task = tokio::task::spawn_local(async move {
                    let mut sink = sink;
                    let client = TcpStream::connect(addr).await.unwrap();
                    let mut client_sink = FramedWrite::new(client, LengthDelimitedCodec::new());
                    SinkExt::send(&mut client_sink, bytes::Bytes::from("test"))
                        .await
                        .unwrap();
                    SinkExt::send(&mut sink, bytes::Bytes::from("response"))
                        .await
                        .unwrap();
                });

                let stream_task = tokio::task::spawn_local(async move {
                    let mut stream = stream;
                    tokio::task::yield_now().await;
                    stream.next().await
                });

                sink_task.await.unwrap();
                let result = stream_task.await.unwrap();

                let bytes = result.unwrap().unwrap();
                assert_eq!(&bytes[..], b"test");
            })
            .await;
    }
}
