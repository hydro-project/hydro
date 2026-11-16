//! Tests for [`DemuxMap`](sinktools::demux_map::DemuxMap) and
//! [`LazyDemuxSink`](sinktools::demux_map_lazy::LazyDemuxSink).
//!
//! Covers both item routing and the polling behavior of `poll_ready`/`poll_flush`/`poll_close`
//! across many inner sinks. In particular, these tests pin down that polling continues to visit
//! *all* inner sinks when some return `Poll::Pending` (so that every sink can make progress and
//! register its waker), while still short-circuiting on `Err`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use futures_task::noop_waker_ref;
use sinktools::sink::SinkExt;
use sinktools::{Sink, demux_map, demux_map_lazy, for_each};

/// Number of inner sinks used in the polling tests.
const N: usize = 8;

/// Shared counters recording how many times each `poll_*` method has been called across all
/// [`CountingSink`]s created from a single test.
#[derive(Default, Debug)]
struct Counts {
    ready: Cell<usize>,
    flush: Cell<usize>,
    close: Cell<usize>,
}

/// What a [`CountingSink`] should return from its `poll_*` methods.
#[derive(Clone, Copy, Debug)]
enum Behavior {
    /// Always `Poll::Ready(Ok(()))`.
    Ready,
    /// Always `Poll::Pending`.
    Pending,
    /// `Poll::Pending` on the first call of each `poll_*` method, `Ready(Ok(()))` thereafter.
    PendingOnce,
    /// Always `Poll::Ready(Err(..))`.
    Error,
}

/// A sink which counts polls and returns a result determined by its [`Behavior`].
struct CountingSink {
    counts: Rc<Counts>,
    behavior: Behavior,
    /// For [`Behavior::PendingOnce`]: whether each of `(ready, flush, close)` has been polled yet.
    polled: [bool; 3],
}

impl CountingSink {
    fn new(counts: Rc<Counts>, behavior: Behavior) -> Self {
        Self {
            counts,
            behavior,
            polled: [false; 3],
        }
    }

    fn poll(&mut self, idx: usize, counter: &Cell<usize>) -> Poll<Result<(), &'static str>> {
        counter.set(counter.get() + 1);
        let first = !std::mem::replace(&mut self.polled[idx], true);
        match self.behavior {
            Behavior::Ready => Poll::Ready(Ok(())),
            Behavior::Pending => Poll::Pending,
            Behavior::PendingOnce if first => Poll::Pending,
            Behavior::PendingOnce => Poll::Ready(Ok(())),
            Behavior::Error => Poll::Ready(Err("boom")),
        }
    }
}

impl Sink<()> for CountingSink {
    type Error = &'static str;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let counts = this.counts.clone();
        this.poll(0, &counts.ready)
    }

    fn start_send(self: Pin<&mut Self>, _item: ()) -> Result<(), Self::Error> {
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let counts = this.counts.clone();
        this.poll(1, &counts.flush)
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        let counts = this.counts.clone();
        this.poll(2, &counts.close)
    }
}

/// A demux sink over `N` [`CountingSink`]s, with the shared [`Counts`].
type Demux = (
    Rc<Counts>,
    Box<dyn Sink<(usize, ()), Error = &'static str> + Unpin>,
);

/// Builds a [`DemuxMap`](sinktools::demux_map::DemuxMap) over `N` sinks with the given behavior.
fn make_demux_map(behavior: Behavior) -> Demux {
    let counts = Rc::new(Counts::default());
    let sinks: HashMap<usize, CountingSink> = (0..N)
        .map(|i| (i, CountingSink::new(counts.clone(), behavior)))
        .collect();
    (counts, Box::new(demux_map::<_, _, ()>(sinks)))
}

/// Builds a [`LazyDemuxSink`](sinktools::demux_map_lazy::LazyDemuxSink) and populates it with
/// `N` sinks (via `start_send`) with the given behavior.
fn make_demux_map_lazy(behavior: Behavior) -> Demux {
    let counts = Rc::new(Counts::default());
    let counts_clone = counts.clone();
    let mut demux = demux_map_lazy::<usize, _, (), _>(move |_key: &usize| {
        CountingSink::new(counts_clone.clone(), behavior)
    });
    for i in 0..N {
        Pin::new(&mut demux).start_send((i, ())).unwrap();
    }
    (counts, Box::new(demux))
}

/// Runs one round of `poll_ready`, `poll_flush`, `poll_close` on `sink`, and returns the results.
fn poll_all<S: Sink<(usize, ())> + Unpin + ?Sized>(
    sink: &mut S,
) -> [Poll<Result<(), S::Error>>; 3] {
    let mut cx = Context::from_waker(noop_waker_ref());
    [
        Pin::new(&mut *sink).poll_ready(&mut cx),
        Pin::new(&mut *sink).poll_flush(&mut cx),
        Pin::new(&mut *sink).poll_close(&mut cx),
    ]
}

/// Asserts that each of `(ready, flush, close)` was polled exactly `expected` times.
#[track_caller]
fn assert_counts(counts: &Counts, expected: usize, msg: &str) {
    assert_eq!(expected, counts.ready.get(), "`poll_ready`: {msg}");
    assert_eq!(expected, counts.flush.get(), "`poll_flush`: {msg}");
    assert_eq!(expected, counts.close.get(), "`poll_close`: {msg}");
}

// --------------------------------------------------------------------------------------------
// Polling behavior checks, shared between `DemuxMap` and `LazyDemuxSink`.
// --------------------------------------------------------------------------------------------

/// When every inner sink is `Ready`, every inner sink must be polled exactly once and the overall
/// result must be `Ready(Ok(()))`.
fn check_all_ready(make: fn(Behavior) -> Demux) {
    let (counts, mut demux) = make(Behavior::Ready);

    let [ready, flush, close] = poll_all(&mut *demux);
    assert_eq!(Poll::Ready(Ok(())), ready);
    assert_eq!(Poll::Ready(Ok(())), flush);
    assert_eq!(Poll::Ready(Ok(())), close);

    assert_counts(&counts, N, "should poll every inner sink");
}

/// When every inner sink is `Pending`, the overall result must be `Pending`, and **every** inner
/// sink must still have been polled (no short-circuiting on `Pending`).
///
/// This matters because each inner sink registers the waker only when it is polled. If we stopped
/// at the first `Pending` sink, later sinks would never get a chance to make progress until the
/// first one woke us up, serializing what could otherwise be concurrent progress.
fn check_pending_polls_all_sinks(make: fn(Behavior) -> Demux) {
    let (counts, mut demux) = make(Behavior::Pending);

    let [ready, flush, close] = poll_all(&mut *demux);
    assert!(ready.is_pending());
    assert!(flush.is_pending());
    assert!(close.is_pending());

    assert_counts(
        &counts,
        N,
        "should poll every inner sink even after one returns `Pending`",
    );
}

/// When inner sinks are `Pending` on the first poll and `Ready` afterwards, the first round must be
/// `Pending` (having polled everything), and the second round must be `Ready(Ok(()))`.
fn check_pending_then_ready(make: fn(Behavior) -> Demux) {
    let (counts, mut demux) = make(Behavior::PendingOnce);

    let [ready, flush, close] = poll_all(&mut *demux);
    assert!(ready.is_pending());
    assert!(flush.is_pending());
    assert!(close.is_pending());
    assert_counts(&counts, N, "first round should poll every inner sink");

    let [ready, flush, close] = poll_all(&mut *demux);
    assert_eq!(Poll::Ready(Ok(())), ready);
    assert_eq!(Poll::Ready(Ok(())), flush);
    assert_eq!(Poll::Ready(Ok(())), close);
    assert_counts(
        &counts,
        2 * N,
        "second round should poll every inner sink again",
    );
}

/// When every inner sink returns `Err`, the overall result must be `Ready(Err(..))` and polling
/// must stop at the **first** inner sink (short-circuit on error).
fn check_error_short_circuits(make: fn(Behavior) -> Demux) {
    let (counts, mut demux) = make(Behavior::Error);

    let [ready, flush, close] = poll_all(&mut *demux);
    assert_eq!(Poll::Ready(Err("boom")), ready);
    assert_eq!(Poll::Ready(Err("boom")), flush);
    assert_eq!(Poll::Ready(Err("boom")), close);

    assert_counts(&counts, 1, "should stop at the first `Err`");
}

// --------------------------------------------------------------------------------------------
// DemuxMap
// --------------------------------------------------------------------------------------------

#[tokio::test]
async fn test_demux_map_routing() {
    let collected: Rc<RefCell<Vec<(&str, i32)>>> = Rc::default();
    let make_sink = |name: &'static str| {
        let collected = collected.clone();
        for_each(move |item: i32| collected.borrow_mut().push((name, item)))
    };
    let mut demux = demux_map([("a", make_sink("a")), ("b", make_sink("b"))]);

    demux.send(("a", 1)).await.unwrap();
    demux.send(("b", 2)).await.unwrap();
    demux.send(("a", 3)).await.unwrap();
    demux.close().await.unwrap();

    assert_eq!(&[("a", 1), ("b", 2), ("a", 3)], &**collected.borrow());
}

#[tokio::test]
#[should_panic(expected = "`DemuxMap` missing key \"c\"")]
async fn test_demux_map_missing_key() {
    let make_sink = || for_each(|_: i32| {});
    let mut demux = demux_map([("a", make_sink()), ("b", make_sink())]);
    demux.send(("c", 1)).await.unwrap();
}

#[test]
fn test_demux_map_all_ready() {
    check_all_ready(make_demux_map);
}

#[test]
fn test_demux_map_pending_polls_all_sinks() {
    check_pending_polls_all_sinks(make_demux_map);
}

#[test]
fn test_demux_map_pending_then_ready() {
    check_pending_then_ready(make_demux_map);
}

#[test]
fn test_demux_map_error_short_circuits() {
    check_error_short_circuits(make_demux_map);
}

// --------------------------------------------------------------------------------------------
// LazyDemuxSink
// --------------------------------------------------------------------------------------------

#[tokio::test]
async fn test_demux_map_lazy_routing() {
    let collected: Rc<RefCell<Vec<(&str, i32)>>> = Rc::default();
    let created: Rc<RefCell<Vec<&str>>> = Rc::default();
    let mut demux = demux_map_lazy({
        let collected = collected.clone();
        let created = created.clone();
        move |name: &&'static str| {
            created.borrow_mut().push(*name);
            let collected = collected.clone();
            let name = *name;
            for_each(move |item: i32| collected.borrow_mut().push((name, item)))
        }
    });

    demux.send(("a", 1)).await.unwrap();
    demux.send(("b", 2)).await.unwrap();
    demux.send(("a", 3)).await.unwrap();
    demux.close().await.unwrap();

    assert_eq!(&[("a", 1), ("b", 2), ("a", 3)], &**collected.borrow());
    // Each sink is created exactly once, on first use.
    assert_eq!(&["a", "b"], &**created.borrow());
}

#[test]
fn test_demux_map_lazy_all_ready() {
    check_all_ready(make_demux_map_lazy);
}

#[test]
fn test_demux_map_lazy_pending_polls_all_sinks() {
    check_pending_polls_all_sinks(make_demux_map_lazy);
}

#[test]
fn test_demux_map_lazy_pending_then_ready() {
    check_pending_then_ready(make_demux_map_lazy);
}

#[test]
fn test_demux_map_lazy_error_short_circuits() {
    check_error_short_circuits(make_demux_map_lazy);
}
