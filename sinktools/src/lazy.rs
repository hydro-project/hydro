//! [`LazySink`], [`LazySource`], and related items.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::{FutureExt, Sink, SinkExt, Stream, StreamExt};

enum LazySinkState<S, Fut, ThunkFunc, Item> {
    Ready(ThunkFunc),
    Thunkulating(Fut, Option<Item>),
    Thunkulated(S),
    Taken,
}

impl<S, Fut, ThunkFunc, Item> Default for LazySinkState<S, Fut, ThunkFunc, Item> {
    fn default() -> Self {
        Self::Taken
    }
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
    S::Error: From<E>,
    T: Unpin,
{
    fn poll_with_sink_op<Op>(&mut self, cx: &mut Context<'_>, op: Op) -> Poll<Result<(), S::Error>>
    where
        Op: FnOnce(&mut S, &mut Context<'_>) -> Poll<Result<(), S::Error>>,
    {
        if matches!(self.state, LazySinkState::Ready(_) | LazySinkState::Taken) {
            return Poll::Ready(Ok(()));
        }

        if let LazySinkState::Thunkulating(ref mut v, ref mut item_buffer) = self.state {
            match v.poll_unpin(cx) {
                Poll::Ready(Ok(mut sink)) => {
                    if let Some(item) = item_buffer.take()
                        && let Err(err) = sink.start_send_unpin(item)
                    {
                        return Poll::Ready(Err(err));
                    }
                    self.state = LazySinkState::Thunkulated(sink);
                }
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                Poll::Pending => return Poll::Pending,
            }
        }

        let LazySinkState::Thunkulated(sink) = &mut self.state else {
            unreachable!()
        };

        op(sink, cx)
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
        match std::mem::take(&mut this.state) {
            LazySinkState::Ready(thunk) => {
                this.state = LazySinkState::Thunkulating(thunk(), Some(item));
                Ok(())
            }
            LazySinkState::Thunkulated(mut sink) => {
                let result = sink.start_send_unpin(item);
                this.state = LazySinkState::Thunkulated(sink);
                result
            }
            _ => unreachable!(),
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
