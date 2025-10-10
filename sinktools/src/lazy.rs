//! [`LazySink`], [`LazySource`], and related items.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::{FutureExt, Sink, SinkExt, Stream, StreamExt};

enum LazySinkState<S, Fut, ThunkFunc, Item> {
    Ready(ThunkFunc),
    Taken,
    Thunkulating(Fut, Item),
    Thunkulated(S),
}

/// A lazy sink will attempt to get a [`Sink`] using the thunk when the first item is sent into it
pub struct LazySink<ThunkFunc, SinkType, Item, ThunkulatingFutureType> {
    state: LazySinkState<SinkType, ThunkulatingFutureType, ThunkFunc, Item>,
}

impl<F, S, T, Fut, E> LazySink<F, S, T, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
    T: Unpin,
{
    /// Creates a new [`LazySink`]. Thunk should be something callable that returns a future that resolves to a [`Sink`] that the lazy sink will forward items to.
    pub fn new(thunk: F) -> Self {
        Self {
            state: LazySinkState::Ready(thunk),
        }
    }
}

impl<F, S, T, Fut, E> LazySink<F, S, T, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
    S: Sink<T> + Unpin,
    S::Error: Into<E>,
    T: Unpin,
{
    fn poll_with_sink_op<Op>(&mut self, cx: &mut Context<'_>, op: Op) -> Poll<Result<(), E>>
    where
        Op: FnOnce(&mut S, &mut Context<'_>) -> Poll<Result<(), S::Error>>,
    {
        assert!(!matches!(self.state, LazySinkState::Taken));

        if matches!(self.state, LazySinkState::Ready(_)) {
            return Poll::Ready(Ok(()));
        }

        if let LazySinkState::Thunkulating(ref mut v, _) = self.state {
            match std::task::ready!(v.poll_unpin(cx)) {
                Ok(sink) => {
                    let LazySinkState::Thunkulating(_, item) =
                        std::mem::replace(&mut self.state, LazySinkState::Thunkulated(sink))
                    else {
                        unreachable!();
                    };

                    let LazySinkState::Thunkulated(ref mut sink) = self.state else {
                        unreachable!();
                    };

                    if let Err(err) = sink.start_send_unpin(item) {
                        return Poll::Ready(Err(err.into()));
                    }
                }
                Err(err) => return Poll::Ready(Err(err.into())),
            }
        }

        let LazySinkState::Thunkulated(sink) = &mut self.state else {
            unreachable!()
        };

        op(sink, cx).map_err(Into::into)
    }
}

impl<F, S, T, Fut, E> Sink<T> for LazySink<F, S, T, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
    S: Sink<T> + Unpin,
    S::Error: Into<E>,
    T: Unpin,
{
    type Error = E;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        self.get_mut()
            .poll_with_sink_op(cx, |sink, cx| sink.poll_ready_unpin(cx))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let this = self.get_mut();
        match std::mem::replace(&mut this.state, LazySinkState::Taken) {
            LazySinkState::Ready(thunk) => {
                this.state = LazySinkState::Thunkulating(thunk(), item);
                Ok(())
            }
            LazySinkState::Thunkulated(mut sink) => {
                let result = sink.start_send_unpin(item);
                this.state = LazySinkState::Thunkulated(sink);
                result.map_err(Into::into)
            }
            _ => unreachable!(),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        self.get_mut()
            .poll_with_sink_op(cx, |sink, cx| sink.poll_flush_unpin(cx))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        self.get_mut()
            .poll_with_sink_op(cx, |sink, cx| sink.poll_close_unpin(cx))
    }
}

enum LazySourceState<S, Fut, Func> {
    None(Func),
    Taken,
    Thunkulating(Fut),
    Thunkulated(S),
}

/// A lazy source will attempt to acquire a stream using the thunk when the first item is pulled from it
pub struct LazySource<ThunkFunc, StreamType, ThunkulatingFutureType> {
    // thunk: Option<ThunkFunc>,
    state: LazySourceState<StreamType, ThunkulatingFutureType, ThunkFunc>,
}

impl<F, S, Fut, E> LazySource<F, S, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
{
    /// Creates a new [`LazySource`]. Thunk should be something callable that returns a future that resolves to a [`Stream`] that the lazy sink will forward items to.
    pub fn new(thunk: F) -> Self {
        Self {
            state: LazySourceState::None(thunk),
        }
    }
}

impl<F, S, Fut, E> Stream for LazySource<F, S, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if matches!(self.state, LazySourceState::None(_)) {
            let LazySourceState::None(thunk_func) =
                std::mem::replace(&mut self.state, LazySourceState::Taken)
            else {
                unreachable!();
            };

            self.state = LazySourceState::Thunkulating(thunk_func());
        }

        if let LazySourceState::Thunkulating(ref mut fut) = self.state {
            match std::task::ready!(fut.poll_unpin(cx)) {
                Ok(stream) => {
                    self.state = LazySourceState::Thunkulated(stream);
                }
                Err(_) => return Poll::Ready(None),
            }
        }

        let LazySourceState::Thunkulated(ref mut stream) = self.state else {
            unreachable!()
        };

        stream.poll_next_unpin(cx)
    }
}

#[cfg(test)]
mod test {
    use core::cell::RefCell;

    use futures_util::{SinkExt, StreamExt};

    use crate::for_each::ForEach;
    use crate::lazy::{LazySink, LazySource};

    #[tokio::test]
    async fn test_lazy_source() {
        let test_data = b"test";

        let mut lazy_source = LazySource::new(|| {
            Box::pin(async {
                Result::<_, std::io::Error>::Ok(futures_util::stream::iter(vec![
                    test_data.as_slice(),
                ]))
            })
        });

        assert_eq!(lazy_source.next().await.unwrap(), test_data);
    }

    #[tokio::test]
    async fn test_lazy_sink() {
        let test_data = b"test";
        let output = RefCell::new(Vec::new());

        let mut lazy_sink = LazySink::new(|| {
            Box::pin(async {
                Result::<_, std::convert::Infallible>::Ok(ForEach::new(|item| {
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
}
