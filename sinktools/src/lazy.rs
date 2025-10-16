//! [`LazySink`], [`LazySource`], and related items.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, ready};

use futures_util::{FutureExt, Sink, Stream};
use pin_project_lite::pin_project;

pin_project! {
    #[project = LazySinkProj]
    enum LazySinkState<Func, Fut, Si, Item> {
        Uninit {
            // Initialization func, always `Some`
            func: Option<Func>,
        },
        Thunkulating {
            // Initialization future.
            #[pin]
            future: Fut,
             // First item sent, always `Some`
            item: Option<Item>,
        },
        Done {
            // The final sink.
            #[pin]
            sink: Si,
            // First item sent, then after always `None`.
            buf: Option<Item>,
        },
    }
}

pin_project! {
    /// A lazy sink will attempt to get a [`Sink`] using the init `Func` when the first item is sent into it.
    pub struct LazySink<Func, Fut, Si, Item> {
        #[pin]
        state: LazySinkState<Func, Fut, Si, Item>,
    }
}

impl<Func, Fut, Si, Item, Error> LazySink<Func, Fut, Si, Item>
where
    Func: FnOnce() -> Fut,
    Fut: Future<Output = Result<Si, Error>>,
    Si: Sink<Item>,
    Error: From<Si::Error>,
{
    /// Creates a new `LazySink` with the given initialization `func`.
    pub fn new(func: Func) -> Self {
        Self {
            state: LazySinkState::Uninit { func: Some(func) },
        }
    }

    fn poll_sink_op(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        sink_op: impl FnOnce(Pin<&mut Si>, &mut Context<'_>) -> Poll<Result<(), Si::Error>>,
    ) -> Poll<Result<(), Error>> {
        let mut this = self.project();

        if let LazySinkProj::Uninit { func: _ } = this.state.as_mut().project() {
            return Poll::Ready(Ok(())); // Lazy
        }

        if let LazySinkProj::Thunkulating { mut future, item } = this.state.as_mut().project() {
            let sink = ready!(future.poll_unpin(cx))?;
            let buf = Some(item.take().unwrap());
            this.state.as_mut().set(LazySinkState::Done { sink, buf });
        }

        if let LazySinkProj::Done { mut sink, buf } = this.state.as_mut().project() {
            if buf.is_some() {
                let () = ready!(sink.as_mut().poll_ready(cx)?);
                let () = sink.as_mut().start_send(buf.take().unwrap())?;
            }
            return (sink_op)(sink, cx).map_err(From::from);
        }

        panic!("`LazySink` in invalid state.");
    }
}

impl<Func, Fut, Si, Item, Error> Sink<Item> for LazySink<Func, Fut, Si, Item>
where
    Func: FnOnce() -> Fut,
    Fut: Future<Output = Result<Si, Error>>,
    Si: Sink<Item>,
    Error: From<Si::Error>,
{
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sink_op(cx, Sink::poll_ready)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let mut this = self.project();

        if let LazySinkProj::Uninit { func } = this.state.as_mut().project() {
            let func = func.take().unwrap();
            let future = (func)();
            let item = Some(item);
            this.state
                .as_mut()
                .set(LazySinkState::Thunkulating { future, item });
            Ok(())
        } else if let LazySinkProj::Done { sink, buf: _buf } = this.state.project() {
            debug_assert!(_buf.is_none());
            sink.start_send(item).map_err(From::from)
        } else {
            panic!("`LazySink` not ready.");
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sink_op(cx, Sink::poll_flush)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_sink_op(cx, Sink::poll_close)
    }
}

pin_project! {
    #[project = LazySourceProj]
    enum LazySourceState<St, Fut, Func> {
        Uninit {
            // Initialization func, always `Some`.
            func: Option<Func>,
        },
        Thunkulating {
            // Initialization future.
            #[pin]
            fut: Fut
        },
        Done {
            #[pin]
            stream: St,
        },
    }
}

pin_project! {
    /// A lazy source will attempt to acquire a stream using the thunk when the first item is pulled from it
    pub struct LazySource<ThunkFunc, StreamType, PreparingFutureType> {
        #[pin]
        state: LazySourceState<StreamType, PreparingFutureType, ThunkFunc>,
    }
}

impl<F, S, Fut, E> LazySource<F, S, Fut>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<S, E>>,
{
    /// Creates a new [`LazySource`]. Thunk should be something callable that returns a future that resolves to a [`Stream`] that the lazy sink will forward items to.
    pub fn new(thunk: F) -> Self {
        Self {
            state: LazySourceState::Uninit { func: Some(thunk) },
        }
    }
}

impl<F, S, Fut, E> Stream for LazySource<F, S, Fut>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<S, E>>,
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let LazySourceProj::Uninit { func } = self.as_mut().project().state.project() {
            let func = func.take().unwrap();
            self.as_mut()
                .project()
                .state
                .set(LazySourceState::Thunkulating { fut: func() });
        }

        if let LazySourceProj::Thunkulating { fut } = self.as_mut().project().state.project() {
            match ready!(fut.poll(cx)) {
                Ok(stream) => {
                    self.as_mut()
                        .project()
                        .state
                        .set(LazySourceState::Done { stream });
                }
                Err(_e) => {
                    // TODO(mingwei): handle errors.
                    return Poll::Ready(None);
                }
            }
        }

        if let LazySourceProj::Done { stream } = self.as_mut().project().state.project() {
            return stream.poll_next(cx);
        }

        panic!("`LazySource` in invalid state.");
    }
}

#[cfg(test)]
mod test {
    use core::cell::RefCell;
    use core::convert::Infallible;
    use core::pin::pin;
    use core::task::Context;

    use futures_util::{Sink, SinkExt, StreamExt};

    use super::*;
    use crate::for_each::ForEach;

    #[tokio::test]
    async fn test_lazy_sink() {
        let test_data = b"test";
        let output = RefCell::new(Vec::new());

        let mut lazy_sink = LazySink::new(|| {
            Box::pin(async {
                Result::<_, Infallible>::Ok(ForEach::new(|item| {
                    output.borrow_mut().extend_from_slice(item);
                }))
            })
        });

        SinkExt::send(&mut lazy_sink, test_data.as_slice())
            .await
            .unwrap();

        SinkExt::send(&mut lazy_sink, test_data.as_slice())
            .await
            .unwrap();

        SinkExt::flush(&mut lazy_sink).await.unwrap();

        SinkExt::close(&mut lazy_sink).await.unwrap();

        assert_eq!(&output.borrow().as_slice()[0..test_data.len()], test_data);
        assert_eq!(&output.borrow().as_slice()[test_data.len()..], test_data);
    }

    #[test]
    fn test_lazy_sink_fut_err() {
        enum DummySink {}
        impl Sink<()> for DummySink {
            type Error = &'static str;
            fn poll_ready(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<(), Self::Error>> {
                panic!()
            }
            fn start_send(self: Pin<&mut Self>, _item: ()) -> Result<(), Self::Error> {
                panic!()
            }
            fn poll_flush(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<(), Self::Error>> {
                panic!()
            }
            fn poll_close(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<Result<(), Self::Error>> {
                panic!()
            }
        }

        let mut lazy_sink = pin!(LazySink::new(|| async {
            Result::<DummySink, _>::Err("Fail!")
        }));

        let cx = &mut Context::from_waker(futures_task::noop_waker_ref());

        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_ready(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_flush(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_close(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_ready(cx));
        assert_eq!(Ok(()), lazy_sink.as_mut().start_send(())); // Works because item is buffered.
        assert_eq!(Poll::Ready(Err("Fail!")), lazy_sink.as_mut().poll_flush(cx)); // Now anything fails.
    }

    #[test]
    fn test_lazy_sink_sink_err() {
        let mut lazy_sink = pin!(LazySink::new(|| async {
            Ok(futures_util::sink::unfold((), |(), _item| async {
                Err("Fail!")
            }))
        }));

        let cx = &mut Context::from_waker(futures_task::noop_waker_ref());

        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_ready(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_flush(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_close(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_ready(cx));
        assert_eq!(Ok(()), lazy_sink.as_mut().start_send(())); // Works because item is buffered.
        assert_eq!(Poll::Ready(Err("Fail!")), lazy_sink.as_mut().poll_flush(cx)); // Now anything fails.
    }

    #[test]
    fn test_lazy_sink_good() {
        let test_data = b"test";

        let mut lazy_sink = pin!(LazySink::new(|| async {
            Result::<_, ()>::Ok(futures_util::sink::unfold((), |(), item| async move {
                assert_eq!(item, test_data);
                Ok(())
            }))
        }));

        let cx = &mut Context::from_waker(futures_task::noop_waker_ref());

        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_ready(cx));
        assert_eq!(Ok(()), lazy_sink.as_mut().start_send(test_data.as_slice()));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_flush(cx));
        assert_eq!(Ok(()), lazy_sink.as_mut().start_send(test_data.as_slice()));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_flush(cx));
        assert_eq!(Poll::Ready(Ok(())), lazy_sink.as_mut().poll_close(cx));
    }

    #[tokio::test]
    async fn test_lazy_source() {
        let test_data = b"test";

        let mut lazy_source = LazySource::new(|| {
            Box::pin(async {
                Result::<_, Infallible>::Ok(futures_util::stream::iter(vec![test_data.as_slice()]))
            })
        });

        assert_eq!(lazy_source.next().await.unwrap(), test_data);
    }

    #[test]
    fn test_lazy_source_err() {
        let mut lazy_source = pin!(LazySource::new(|| async {
            Result::<futures_util::stream::Empty<()>, _>::Err("Fail!")
        }));

        let cx = &mut Context::from_waker(futures_task::noop_waker_ref());

        assert_eq!(Poll::Ready(None), lazy_source.as_mut().poll_next(cx));
    }

    #[test]
    fn test_lazy_source_good() {
        let test_data = b"test";

        let mut lazy_source = pin!(LazySource::new(|| async {
            Result::<_, Infallible>::Ok(futures_util::stream::iter(test_data))
        }));

        let cx = &mut Context::from_waker(futures_task::noop_waker_ref());

        assert_eq!(Poll::Ready(Some(&b't')), lazy_source.as_mut().poll_next(cx));
        assert_eq!(Poll::Ready(Some(&b'e')), lazy_source.as_mut().poll_next(cx));
        assert_eq!(Poll::Ready(Some(&b's')), lazy_source.as_mut().poll_next(cx));
        assert_eq!(Poll::Ready(Some(&b't')), lazy_source.as_mut().poll_next(cx));
        assert_eq!(Poll::Ready(None), lazy_source.as_mut().poll_next(cx));
    }
}
