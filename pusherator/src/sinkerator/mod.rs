use std::pin::Pin;
use std::task::{Context, Poll};

/// A `Sinkerator` is a value into which other values can be sent, asynchronously.
///
/// Provides the same functionality as [`futures::Sink`] but with a slightly
/// simplified API, which avoids the "pre-flight" polling of [`futures::Sink::poll_ready`].
pub trait Sinkerator<Item> {
    /// The type of value produced by the sink when an error occurs.
    type Error;

    /// Sends an item to the sink.
    ///
    /// If this method does not return `Poll::Ready(Ok(()))`, this sink is considered to be in a
    /// pending state, and this method may be called again repeatedly with `None`. Once
    /// `Poll::Ready(Ok(())` is returned, the sink is in a ready state and another item `Some` may
    /// be sent.
    ///
    /// Calling this method with `None` when the sink is already ready is allowed but discouraged.
    ///
    /// In most cases, if the sink encounters an error, the sink will permanently be unable to
    /// receive items.
    fn poll_send(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        item: Option<Item>,
    ) -> Poll<Result<(), Self::Error>>;

    /// Flush any remaining output from this sink.
    ///
    /// If the sink is in a pending state (`poll_send` did not return `Poll::Ready(Ok(()))`),
    /// then pending items may fail to flush, though they will remain within the sink.
    ///
    /// Returns `Poll::Ready(Ok(()))` when no buffered items remain. If this
    /// value is returned then it is guaranteed that all previous values sent
    /// via `start_send` have been flushed.
    ///
    /// Returns `Poll::Pending` if there is more work left to do, in which
    /// case the current task is scheduled (via `cx.waker().wake_by_ref()`) to wake up when
    /// `poll_flush` should be called again.
    ///
    /// In most cases, if the sink encounters an error, the sink will
    /// permanently be unable to receive items.
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;

    /// Flush any remaining output and close this sink, if necessary.
    ///
    /// If the sink is in a pending state (`poll_send` did not return `Poll::Ready(Ok(()))`),
    /// then pending items may fail to flush and will be dropped.
    ///
    /// Returns `Poll::Ready(Ok(()))` when no buffered items remain and the sink
    /// has been successfully closed.
    ///
    /// Returns `Poll::Pending` if there is more work left to do, in which
    /// case the current task is scheduled (via `cx.waker().wake_by_ref()`) to wake up when
    /// `poll_close` should be called again.
    ///
    /// If this function encounters an error, the sink should be considered to
    /// have failed permanently, and no more `Sink` methods should be called.
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
}

mod filter;
mod filter_map;
mod flat_map;
mod flatten;
mod for_each;
mod inspect;
mod map;
mod pivot;
mod sink_compat;
mod tee;
mod unzip;
pub use filter::Filter;
pub use filter_map::FilterMap;
pub use flat_map::FlatMap;
pub use flatten::Flatten;
pub use for_each::ForEach;
pub use inspect::Inspect;
pub use map::Map;
pub use pivot::Pivot;
pub use sink_compat::SinkCompat;
pub use tee::Tee;
pub use unzip::Unzip;
