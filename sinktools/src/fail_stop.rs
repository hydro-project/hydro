//! [`FailStopSink`] and related items.
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::Sink;

/// A [`Sink`] wrapper implementing fail-stop semantics for network channels.
///
/// While the inner sink is healthy, all operations are forwarded to it unchanged. The first
/// error returned by the inner sink permanently marks the channel as failed: the inner sink is
/// dropped (disconnecting the underlying connection; there is no re-dial) and the `on_fail`
/// callback is invoked exactly once (typically to log the failure). From then on the wrapper is
/// a black hole: sends succeed immediately and items are silently discarded.
///
/// This matches the fail-stop failure model, where a failed connection stops all future
/// deliveries to that recipient (modeling the recipient as having failed) without affecting the
/// sender. Because the wrapper never returns an error, a dead peer cannot crash the sending
/// process, and when used per-destination under a demux (e.g. [`crate::demux_map()`]) it cannot
/// poison deliveries to healthy sibling destinations through the demux's shared error path.
pub struct FailStopSink<Si, F> {
    /// The inner sink. `None` once the channel has failed; dropping it disconnects.
    sink: Option<Si>,
    /// Failure callback. `None` once invoked (the channel fails at most once).
    on_fail: Option<F>,
}

impl<Si, F> FailStopSink<Si, F> {
    /// Wraps `sink`. On the first error, `on_fail` is invoked exactly once with the name of the
    /// failing sink operation (`"poll_ready"`, `"start_send"`, `"poll_flush"`, or
    /// `"poll_close"`) and the error.
    pub fn new(sink: Si, on_fail: F) -> Self {
        Self {
            sink: Some(sink),
            on_fail: Some(on_fail),
        }
    }

    /// Returns `true` if the channel has failed and is now discarding all messages.
    pub fn is_dead(&self) -> bool {
        self.sink.is_none()
    }

    /// Marks the channel as failed, invoking `on_fail` and dropping the inner sink
    /// (disconnect).
    fn fail<E>(&mut self, during: &'static str, err: &E)
    where
        F: FnOnce(&'static str, &E),
    {
        if let Some(on_fail) = self.on_fail.take() {
            (on_fail)(during, err);
        }
        self.sink = None;
    }
}

impl<Si, Item, F> Sink<Item> for FailStopSink<Si, F>
where
    Si: Sink<Item> + Unpin,
    F: FnOnce(&'static str, &Si::Error) + Unpin,
{
    /// Same type as the inner sink's error for drop-in compatibility, but never actually
    /// returned: errors are converted into the fail-stop state instead.
    type Error = Si::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let Some(sink) = this.sink.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        match Pin::new(sink).poll_ready(cx) {
            Poll::Ready(Err(err)) => {
                this.fail("poll_ready", &err);
                Poll::Ready(Ok(()))
            }
            poll => poll,
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        let this = self.get_mut();
        let Some(sink) = this.sink.as_mut() else {
            // Black hole: the item is silently discarded.
            return Ok(());
        };
        if let Err(err) = Pin::new(sink).start_send(item) {
            this.fail("start_send", &err);
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let Some(sink) = this.sink.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        match Pin::new(sink).poll_flush(cx) {
            Poll::Ready(Err(err)) => {
                this.fail("poll_flush", &err);
                Poll::Ready(Ok(()))
            }
            poll => poll,
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let Some(sink) = this.sink.as_mut() else {
            return Poll::Ready(Ok(()));
        };
        match Pin::new(sink).poll_close(cx) {
            Poll::Ready(Err(err)) => {
                this.fail("poll_close", &err);
                Poll::Ready(Ok(()))
            }
            poll => poll,
        }
    }
}

#[cfg(test)]
mod test {
    use core::cell::Cell;
    use core::pin::pin;

    use futures_util::SinkExt;

    use super::*;

    /// Sink that accepts `ok_sends` items and then errors forever.
    struct FlakySink {
        ok_sends: usize,
        received: Vec<u32>,
    }
    impl Sink<u32> for FlakySink {
        type Error = &'static str;
        fn poll_ready(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn start_send(self: Pin<&mut Self>, item: u32) -> Result<(), Self::Error> {
            let this = self.get_mut();
            if 0 < this.ok_sends {
                this.ok_sends -= 1;
                this.received.push(item);
                Ok(())
            } else {
                Err("peer died")
            }
        }
        fn poll_flush(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_fail_stop_black_holes_after_error() {
        let fail_count = Cell::new(0);
        let observed = Cell::new(None::<(&'static str, &'static str)>);
        let mut sink = pin!(FailStopSink::new(
            FlakySink {
                ok_sends: 2,
                received: Vec::new(),
            },
            |during, err: &&'static str| {
                fail_count.set(fail_count.get() + 1);
                observed.set(Some((during, err)));
            },
        ));

        // Healthy sends are forwarded; the callback has not fired.
        sink.send(1).await.unwrap();
        sink.send(2).await.unwrap();
        assert!(!sink.is_dead());
        assert_eq!(0, fail_count.get());

        // First error kills the channel but is not surfaced; the callback fires once with the
        // failing operation and the error.
        sink.send(3).await.unwrap();
        assert!(sink.is_dead());
        assert_eq!(1, fail_count.get());
        assert_eq!(Some(("start_send", "peer died")), observed.get());

        // Subsequent operations silently discard, without re-invoking the callback.
        sink.send(4).await.unwrap();
        sink.send(5).await.unwrap();
        sink.flush().await.unwrap();
        sink.close().await.unwrap();
        assert!(sink.is_dead());
        assert_eq!(1, fail_count.get());
    }
}
