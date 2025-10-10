//! [`LazySink`], [`LazySource`], and related items.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::Sink;
use futures_util::SinkExt;
use futures_util::StreamExt;
use futures_util::{FutureExt, Stream};

enum LazySinkState<S, Fut> {
    None,
    Thunkulating(Fut),
    Thunkulated(S),
}

/// A lazy sink will attempt to get a [`Sink`] using the thunk when the first item is sent into it
pub struct LazySink<ThunkFunc, SinkType, Item, ThunkulatingFutureType> {
    thunk: Option<ThunkFunc>,
    item_buffer: Option<Item>,
    state: LazySinkState<SinkType, ThunkulatingFutureType>,
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
            thunk: Some(thunk),
            item_buffer: None,
            state: LazySinkState::None,
        }
    }
}

impl<F, S, T, Fut, E> LazySink<F, S, T, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
    S: Sink<T> + Unpin,
    S::Error: From<E>,
    T: Unpin,
{
    fn poll_with_sink_op<Op>(&mut self, cx: &mut Context<'_>, op: Op) -> Poll<Result<(), S::Error>>
    where
        Op: FnOnce(&mut S, &mut Context<'_>) -> Poll<Result<(), S::Error>>,
    {
        if matches!(self.state, LazySinkState::None) {
            return Poll::Ready(Ok(()));
        }

        if let LazySinkState::Thunkulating(ref mut v) = self.state {
            match v.poll_unpin(cx) {
                Poll::Ready(Ok(sink)) => {
                    self.state = LazySinkState::Thunkulated(sink);
                }
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                Poll::Pending => return Poll::Pending,
            }
        }

        let LazySinkState::Thunkulated(sink) = &mut self.state else {
            unreachable!()
        };

        match sink.poll_ready_unpin(cx) {
            Poll::Ready(Ok(_)) => {
                if let Some(item) = self.item_buffer.take()
                    && let Err(err) = sink.start_send_unpin(item)
                {
                    return Poll::Ready(Err(err));
                }
                op(sink, cx)
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<F, S, T, Fut, E> Sink<T> for LazySink<F, S, T, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
    S: Sink<T> + Unpin,
    S::Error: From<E>,
    T: Unpin,
{
    type Error = S::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .poll_with_sink_op(cx, |sink, cx| sink.poll_ready_unpin(cx))
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        let this = self.get_mut();
        match this.state {
            LazySinkState::None => {
                this.state = LazySinkState::Thunkulating(this.thunk.take().unwrap()());
                this.item_buffer = Some(item);
                Ok(())
            }
            LazySinkState::Thunkulating(_) => unreachable!(),
            LazySinkState::Thunkulated(ref mut sink) => {
                assert!(this.item_buffer.is_none());
                sink.start_send_unpin(item)
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .poll_with_sink_op(cx, |sink, cx| sink.poll_flush_unpin(cx))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut()
            .poll_with_sink_op(cx, |sink, cx| sink.poll_close_unpin(cx))
    }
}

enum LazySourceState<S, Fut> {
    None,
    Thunkulating(Fut),
    Thunkulated(S),
}

/// A lazy source will attempt to acquire a stream using the thunk when the first item is pulled from it
pub struct LazySource<ThunkFunc, StreamType, ThunkulatingFutureType> {
    thunk: Option<ThunkFunc>,
    state: LazySourceState<StreamType, ThunkulatingFutureType>,
}

impl<F, S, Fut, E> LazySource<F, S, Fut>
where
    F: FnOnce() -> Fut + Unpin,
    Fut: Future<Output = Result<S, E>> + Unpin,
{
    /// Creates a new [`LazySource`]. Thunk should be something callable that returns a future that resolves to a [`Stream`] that the lazy sink will forward items to.
    pub fn new(thunk: F) -> Self {
        Self {
            thunk: Some(thunk),
            state: LazySourceState::None,
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
        if matches!(self.state, LazySourceState::None) {
            self.state = LazySourceState::Thunkulating(self.thunk.take().unwrap()());
        }

        if let LazySourceState::Thunkulating(ref mut fut) = self.state {
            match fut.poll_unpin(cx) {
                Poll::Ready(Ok(stream)) => {
                    self.state = LazySourceState::Thunkulated(stream);
                }
                Poll::Ready(Err(_)) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
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
    use crate::{
        for_each::ForEach,
        lazy::{LazySink, LazySource},
    };
    use core::cell::RefCell;
    use futures_util::{SinkExt, StreamExt};

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

        SinkExt::flush(&mut lazy_sink).await.unwrap();

        SinkExt::close(&mut lazy_sink).await.unwrap();

        assert_eq!(output.borrow().as_slice(), test_data);
    }
}
