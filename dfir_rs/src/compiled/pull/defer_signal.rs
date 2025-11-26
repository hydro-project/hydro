use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::{FusedStream, Stream};
use pin_project_lite::pin_project;

pin_project! {
    /// Special stream for the `defer_signal` operator.
    #[must_use = "streams do nothing unless polled"]
    pub struct DeferSignal<'a, InputStream, SignalStream>
    where
        InputStream: Stream,
        SignalStream: FusedStream,
    {
        #[pin]
        input: InputStream,
        #[pin]
        signal: SignalStream,

        buf: &'a mut VecDeque<InputStream::Item>,

        signalled: bool,
    }
}

impl<'a, InputStream, SignalStream> DeferSignal<'a, InputStream, SignalStream>
where
    InputStream: Stream,
    SignalStream: FusedStream,
{
    /// Creates a new `DeferSignal` stream.
    pub fn new(
        input: InputStream,
        signal: SignalStream,
        buf: &'a mut VecDeque<InputStream::Item>,
    ) -> Self {
        Self {
            input,
            signal,
            buf,
            signalled: false,
        }
    }
}

impl<'a, InputStream, SignalStream> Stream for DeferSignal<'a, InputStream, SignalStream>
where
    InputStream: Stream,
    SignalStream: FusedStream,
{
    type Item = InputStream::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // Stage 1: Exhaust the signal.
        // This will get called every time even after returning EOS (`None`), so we need fused.
        while let Some(_signal) = ready!(this.signal.as_mut().poll_next(cx)) {
            *this.signalled = true;
        }

        // Stage 2: Empty the buffer (if signalled).
        if *this.signalled
            && let Some(item) = this.buf.pop_front()
        {
            return Poll::Ready(Some(item));
        }
        debug_assert_eq!(0, this.buf.len());

        // Stage 3: Exhaust the input stream
        while let Some(item) = ready!(this.input.as_mut().poll_next(cx)) {
            if *this.signalled {
                return Poll::Ready(Some(item));
            } else {
                this.buf.push_back(item);
            }
        }

        // Done
        // Once the input stream returns `None` (above), we return `None`, so no need for input stream to be fused.
        Poll::Ready(None)
    }
}
