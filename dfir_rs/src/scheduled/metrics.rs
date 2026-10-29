//! Runtime metrics for DFIR.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::task::{Context, Poll};

use dfir_lang::graph_ids::{GraphNodeId, GraphSubgraphId};
use pin_project_lite::pin_project;
use sealed::sealed;
use slotmap::SecondaryMap;
use web_time::{Duration, Instant};

/// Typestate marker for whether a [`Context`](super::context::Context) tracks runtime metrics.
///
/// This trait is sealed and is implemented for exactly two types:
///
/// - [`DfirMetrics`]: metrics are tracked. The context holds a shared [`Arc<DfirMetrics>`] handle
///   which is updated by the generated code and exposed via
///   [`Dfir::metrics`](super::context::Dfir::metrics).
/// - `()`: metrics are not tracked. The handle is the zero-sized `()`, so the context statically
///   contains no metrics state at all, and the `metrics()` accessors do not exist.
///
/// Which one is used is determined at codegen time by
/// [`AsCodeOptions::include_metrics_tracking`](dfir_lang::graph::AsCodeOptions::include_metrics_tracking),
/// so there is no runtime check or default-allocated metrics when tracking is disabled.
#[sealed]
pub trait MetricsState: 'static {
    /// The metrics handle stored inside the context: [`Arc<DfirMetrics>`] or `()`.
    type Handle;
}

/// Metrics are tracked.
#[sealed]
impl MetricsState for DfirMetrics {
    type Handle = Arc<DfirMetrics>;
}

/// Metrics are not tracked. Zero-sized, no runtime cost.
#[sealed]
impl MetricsState for () {
    type Handle = ();
}

/// Metrics for a [`Dfir`](super::context::Dfir) graph instance.
///
/// Call [`Dfir::metrics`](super::context::Dfir::metrics) for reference-counted continually-updated metrics,
/// or call [`Dfir::metrics_intervals`](super::context::Dfir::metrics_intervals) to obtain a [`DfirMetricsIntervals`] handle, and use
/// [`DfirMetricsIntervals::take_interval`] to retrieve metrics for successive intervals.
///
/// # Thread safety
///
/// All counters are atomics, so the live metrics may be read from any thread (e.g. a metrics
/// reporting task) while the dataflow is running. Each individual counter is always consistent, but
/// a snapshot (via [`Clone`] or [`DfirMetricsIntervals::take_interval`]) taken from another thread
/// may observe counters from slightly different moments in time relative to each other (e.g.
/// [`SubgraphMetrics::total_run_count`] incremented but [`SubgraphMetrics::total_poll_duration`]
/// not yet updated for that run).
#[derive(Default, Clone)]
#[non_exhaustive]
pub struct DfirMetrics {
    /// Per-subgraph metrics.
    pub subgraphs: SecondaryMap<GraphSubgraphId, SubgraphMetrics>,
    /// Per-handoff metrics.
    pub handoffs: SecondaryMap<GraphNodeId, HandoffMetrics>,
}

impl DfirMetrics {
    /// Subtracts `other` from self.
    pub(super) fn diff(&mut self, other: &Self) {
        for (sg_id, prev_sg_metrics) in other.subgraphs.iter() {
            if let Some(curr_sg_metrics) = self.subgraphs.get_mut(sg_id) {
                curr_sg_metrics.diff(prev_sg_metrics);
            }
        }
        for (handoff_id, prev_handoff_metrics) in other.handoffs.iter() {
            if let Some(curr_handoff_metrics) = self.handoffs.get_mut(handoff_id) {
                curr_handoff_metrics.diff(prev_handoff_metrics);
            }
        }
    }
}

/// A handle into a DFIR instance's metrics, where each call to [`Self::take_interval`] ends the current interval and
/// returns its metrics. Obtained via [`Dfir::metrics_intervals`](super::context::Dfir::metrics_intervals).
///
/// The first call to `take_interval` returns metrics since this DFIR instance was created. Each subsequent call to
/// `take_interval` returns metrics since the previous call.
///
/// Cloning the handle "forks" it from the original, as afterwards each interval may return different metrics
/// depending on when exactly `take_interval` is called.
#[derive(Clone)]
pub struct DfirMetricsIntervals {
    /// `curr` is continually updating (via shared ownership).
    pub(super) curr: Arc<DfirMetrics>,
    /// `prev` is an unchanging snapshot in time. `None` for "since creation".
    pub(super) prev: Option<DfirMetrics>,
}

impl DfirMetricsIntervals {
    /// Ends the current interval and returns the accumulated metrics across the interval.
    ///
    /// The first call to `take_interval` returns metrics since this DFIR instance was created. Each subsequent call to
    /// `take_interval` returns metrics since the previous call.
    pub fn take_interval(&mut self) -> DfirMetrics {
        let mut curr = self.curr.as_ref().clone();
        if let Some(prev) = self.prev.replace(curr.clone()) {
            curr.diff(&prev);
        }
        curr
    }

    /// Returns a reference-counted handle to the original continually-updated runtime metrics for this DFIR instance.
    ///
    /// See [`Dfir::metrics`](super::context::Dfir::metrics).
    pub fn all_metrics(&self) -> Arc<DfirMetrics> {
        Arc::clone(&self.curr)
    }
}

/// An atomic cell holding a single metric value: [`AtomicCount`] or [`AtomicDuration`].
///
/// Metrics are written only by the thread running the dataflow and may be read concurrently from
/// other threads, so all operations use [`Ordering::Relaxed`].
#[sealed]
pub trait MetricCell: Default {
    /// The plain value type stored in this cell.
    type Value: Copy;

    /// Reads the current value.
    fn get(&self) -> Self::Value;
    /// Overwrites the current value.
    fn set(&self, value: Self::Value);
    /// Adds `value` to the current value.
    fn add(&self, value: Self::Value);
}

/// An atomic `usize` counter metric. See [`MetricCell`].
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct AtomicCount(AtomicUsize);

#[sealed]
impl MetricCell for AtomicCount {
    type Value = usize;

    fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    fn set(&self, value: usize) {
        self.0.store(value, Ordering::Relaxed);
    }

    fn add(&self, value: usize) {
        self.0.fetch_add(value, Ordering::Relaxed);
    }
}

impl Clone for AtomicCount {
    fn clone(&self) -> Self {
        let cell = Self::default();
        cell.set(self.get());
        cell
    }
}

/// An atomic [`Duration`] metric, stored as whole nanoseconds in a `u64` (saturating at ~584
/// years). See [`MetricCell`].
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct AtomicDuration(AtomicU64);

impl AtomicDuration {
    fn to_nanos(value: Duration) -> u64 {
        u64::try_from(value.as_nanos()).unwrap_or(u64::MAX)
    }
}

#[sealed]
impl MetricCell for AtomicDuration {
    type Value = Duration;

    fn get(&self) -> Duration {
        Duration::from_nanos(self.0.load(Ordering::Relaxed))
    }

    fn set(&self, value: Duration) {
        self.0.store(Self::to_nanos(value), Ordering::Relaxed);
    }

    fn add(&self, value: Duration) {
        self.0.fetch_add(Self::to_nanos(value), Ordering::Relaxed);
    }
}

impl Clone for AtomicDuration {
    fn clone(&self) -> Self {
        let cell = Self::default();
        cell.set(self.get());
        cell
    }
}

/// Declarative macro to generate metrics structs with atomic cell fields and value getter methods.
macro_rules! define_metrics {
    (
        $(#[$struct_attr:meta])*
        pub struct $struct_name:ident {
            $(
                $( #[doc = $doc:literal] )*
                #[diff($diff:ident)]
                $( #[$field_attr:meta] )*
                $field_name:ident: $field_type:ty,
            )*
        }
    ) => {
        $(#[$struct_attr])*
        #[derive(Default, Debug, Clone)]
        #[non_exhaustive] // May add more metrics later.
        pub struct $struct_name {
            $(
                #[doc(hidden)] // Public for codegen access; use the getter method instead.
                $(#[$field_attr])*
                pub $field_name: $field_type,
            )*
        }

        impl $struct_name {
            $(
                $( #[doc = $doc] )*
                pub fn $field_name(&self) -> <$field_type as MetricCell>::Value {
                    self.$field_name.get()
                }
            )*

            fn diff(&mut self, other: &Self) {
                $(
                    define_metrics_diff_field!($diff, $field_name, self, other);
                )*
            }
        }
    };
}

macro_rules! define_metrics_diff_field {
    (total, $field:ident, $slf:ident, $other:ident) => {
        debug_assert!($other.$field() <= $slf.$field());
        $slf.$field.set($slf.$field() - $other.$field());
    };
    (curr, $field:ident, $slf:ident, $other:ident) => {};
}

define_metrics! {
    /// Per-handoff metrics.
    pub struct HandoffMetrics {
        /// Number of items currently in the handoff.
        #[diff(curr)]
        curr_items_count: AtomicCount,

        /// Total number of items read out of the handoff.
        #[diff(total)]
        total_items_count: AtomicCount,
    }
}

define_metrics! {
    /// Per-subgraph metrics.
    pub struct SubgraphMetrics {
        /// Number of times the subgraph has run.
        #[diff(total)]
        total_run_count: AtomicCount,

        /// Time elapsed during polling (when the subgraph is actively doing work).
        #[diff(total)]
        total_poll_duration: AtomicDuration,

        /// Number of times the subgraph has been polled.
        #[diff(total)]
        total_poll_count: AtomicCount,

        /// Time elapsed during idle (when the subgraph has yielded and is waiting for async events).
        #[diff(total)]
        total_idle_duration: AtomicDuration,

        /// Number of times the subgraph has been idle.
        #[diff(total)]
        total_idle_count: AtomicCount,
    }
}

pin_project! {
    /// Helper struct which instruments a future to track polling times.
    #[doc(hidden)]
    pub struct InstrumentSubgraph<'a, Fut> {
        #[pin]
        future: Fut,
        idle_start: Option<Instant>,
        metrics: &'a SubgraphMetrics,
    }
}

impl<'a, Fut> InstrumentSubgraph<'a, Fut> {
    /// Wrap a future to track per-subgraph poll and idle durations.
    pub fn new(future: Fut, metrics: &'a SubgraphMetrics) -> Self {
        Self {
            future,
            idle_start: None,
            metrics,
        }
    }
}

impl<'a, Fut> Future for InstrumentSubgraph<'a, Fut>
where
    Fut: Future,
{
    type Output = Fut::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // End idle duration.
        if let Some(idle_start) = this.idle_start {
            this.metrics.total_idle_duration.add(idle_start.elapsed());
            this.metrics.total_idle_count.add(1);
        }

        // Begin poll duration.
        let poll_start = Instant::now();
        let out = this.future.poll(cx);

        // End poll duration.
        this.metrics.total_poll_duration.add(poll_start.elapsed());
        this.metrics.total_poll_count.add(1);

        // Begin idle duration.
        this.idle_start.replace(Instant::now());

        out
    }
}

#[cfg(test)]
mod test {
    use dfir_lang::graph_ids::{GraphNodeId, GraphSubgraphId};
    use slotmap::SlotMap;

    use super::*;

    #[test]
    fn test_dfir_metrics_intervals() {
        // Create slotmaps to generate valid keys.
        let mut sg_map: SlotMap<GraphSubgraphId, ()> = SlotMap::with_key();
        let mut node_map: SlotMap<GraphNodeId, ()> = SlotMap::with_key();
        let sg_id = sg_map.insert(());
        let handoff_id = node_map.insert(());

        let mut metrics = DfirMetrics::default();
        let sg_metrics = SubgraphMetrics::default();
        sg_metrics.total_run_count.set(5);
        sg_metrics.total_poll_count.set(10);
        sg_metrics.total_idle_count.set(2);
        sg_metrics
            .total_poll_duration
            .set(Duration::from_millis(500));
        sg_metrics
            .total_idle_duration
            .set(Duration::from_millis(200));
        metrics.subgraphs.insert(sg_id, sg_metrics);
        let hoff_metrics = HandoffMetrics::default();
        hoff_metrics.curr_items_count.set(3);
        hoff_metrics.total_items_count.set(100);
        metrics.handoffs.insert(handoff_id, hoff_metrics);
        let metrics = Arc::new(metrics);

        let mut intervals = DfirMetricsIntervals {
            curr: Arc::clone(&metrics),
            prev: None,
        };

        // First iteration - captures initial state
        let first = intervals.take_interval();
        let sg_metrics = &first.subgraphs[sg_id];
        assert_eq!(sg_metrics.total_run_count(), 5);
        let hoff_metrics = &first.handoffs[handoff_id];
        assert_eq!(hoff_metrics.total_items_count(), 100);
        assert_eq!(hoff_metrics.curr_items_count(), 3);

        // Simulate more work being done.
        let sg_metrics = &metrics.subgraphs[sg_id];
        sg_metrics.total_run_count.set(12);
        sg_metrics.total_poll_count.set(25);
        sg_metrics.total_idle_count.set(7);
        sg_metrics
            .total_poll_duration
            .set(Duration::from_millis(1200));
        sg_metrics
            .total_idle_duration
            .set(Duration::from_millis(600));
        let hoff_metrics = &metrics.handoffs[handoff_id];
        hoff_metrics.total_items_count.set(250);
        hoff_metrics.curr_items_count.set(10);

        // Second iteration - should return the diff
        let second = intervals.take_interval();
        let sg_metrics = &second.subgraphs[sg_id];
        assert_eq!(sg_metrics.total_run_count(), 7); // 12 - 5
        assert_eq!(sg_metrics.total_poll_count(), 15); // 25 - 10
        assert_eq!(sg_metrics.total_idle_count(), 5); // 7 - 2
        //
        let hoff_metrics = &second.handoffs[handoff_id];
        // total_items_count should be diffed
        assert_eq!(hoff_metrics.total_items_count(), 150); // 250 - 100
        // curr_items_count should NOT be diffed (it's a current value, not cumulative)
        assert_eq!(hoff_metrics.curr_items_count(), 10);
    }
}
