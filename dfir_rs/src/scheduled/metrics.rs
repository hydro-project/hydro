//! Runtime metrics for DFIR.

use std::cell::Cell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use web_time::{Duration, Instant};

#[expect(unused_imports, reason = "used for rustdoc links")]
use super::graph::Dfir;
use super::{HandoffTag, SubgraphTag};
use crate::util::slot_vec::SecondarySlotVec;

/// Metrics for a [`Dfir`] graph instance.
///
/// Call [`Dfir::metrics`] for reference-counted continually-updated metrics,
/// or call [`Dfir::metrics_intervals`] to obtain a [`DfirMetricsIntervals`] handle, and use
/// [`DfirMetricsIntervals::take_interval`] to retrieve metrics for successive intervals.
#[derive(Default, Clone)]
#[non_exhaustive]
pub struct DfirMetrics {
    /// Per-subgraph metrics.
    pub subgraphs: SecondarySlotVec<SubgraphTag, SubgraphMetrics>,
    /// Per-handoff metrics.
    pub handoffs: SecondarySlotVec<HandoffTag, HandoffMetrics>,
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
/// returns its metrics. Obtained via [`Dfir::metrics_intervals`].
///
/// The first call to `take_interval` returns metrics since this DFIR instance was created. Each subsequent call to
/// `take_interval` returns metrics since the previous call.
///
/// Cloning the handle "forks" it from the original, as afterwards each interval may return different metrics
/// depending on when exactly `take_interval` is called.
#[derive(Clone)]
pub struct DfirMetricsIntervals {
    /// `curr` is continually updating (via shared ownership).
    pub(super) curr: Rc<DfirMetrics>,
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
    /// See [`Dfir::metrics`].
    pub fn all_metrics(&self) -> Rc<DfirMetrics> {
        Rc::clone(&self.curr)
    }
}

/// Declarative macro to generate metrics structs with Cell-based fields and getter methods.
macro_rules! define_metrics {
    (
        $(#[$struct_attr:meta])*
        pub struct $struct_name:ident {
            $(
                $( #[doc = $doc:literal] )*
                #[diff($diff:ident)]
                $( #[$field_attr:meta] )*
                $field_vis:vis $field_name:ident: Cell<$field_type:ty>,
            )*
        }
    ) => {
        $(#[$struct_attr])*
        #[derive(Default, Debug, Clone)]
        #[non_exhaustive] // May add more metrics later.
        pub struct $struct_name {
            $(
                $(#[$field_attr])*
                $field_vis $field_name: Cell<$field_type>,
            )*
        }

        impl $struct_name {
            $(
                $( #[doc = $doc] )*
                pub fn $field_name(&self) -> $field_type {
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
        debug_assert!($other.$field.get() <= $slf.$field.get());
        $slf.$field.update(|x| x - $other.$field.get());
    };
    (curr, $field:ident, $slf:ident, $other:ident) => {};
}

define_metrics! {
    /// Per-handoff metrics.
    pub struct HandoffMetrics {
        /// Number of items currently in the handoff.
        #[diff(curr)]
        pub(super) curr_items_count: Cell<usize>,

        /// Total number of items read out of the handoff.
        #[diff(total)]
        pub(super) total_items_count: Cell<usize>,
    }
}

define_metrics! {
    /// Per-subgraph metrics.
    pub struct SubgraphMetrics {
        /// Number of times the subgraph has run.
        #[diff(total)]
        pub(super) total_run_count: Cell<usize>,

        /// Time elapsed during polling (when the subgraph is actively doing work).
        #[diff(total)]
        pub(super) total_poll_duration: Cell<Duration>,

        /// Number of times the subgraph has been polled.
        #[diff(total)]
        pub(super) total_poll_count: Cell<usize>,

        /// Time elapsed during idle (when the subgraph has yielded and is waiting for async events).
        #[diff(total)]
        pub(super) total_idle_duration: Cell<Duration>,

        /// Number of times the subgraph has been idle.
        #[diff(total)]
        pub(super) total_idle_count: Cell<usize>,
    }
}

pin_project! {
    /// Helper struct which instruments a future to track polling times.
    pub(crate) struct InstrumentSubgraph<'a, Fut> {
        #[pin]
        future: Fut,
        idle_start: Option<Instant>,
        metrics: &'a SubgraphMetrics,
    }
}

impl<'a, Fut> InstrumentSubgraph<'a, Fut> {
    pub(crate) fn new(future: Fut, metrics: &'a SubgraphMetrics) -> Self {
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
            this.metrics
                .total_idle_duration
                .update(|x| x + idle_start.elapsed());
            this.metrics.total_idle_count.update(|x| x + 1);
        }

        // Begin poll duration.
        let poll_start = Instant::now();
        let out = this.future.poll(cx);

        // End poll duration.
        this.metrics
            .total_poll_duration
            .update(|x| x + poll_start.elapsed());
        this.metrics.total_poll_count.update(|x| x + 1);

        // Begin idle duration.
        this.idle_start.replace(Instant::now());

        out
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::scheduled::{HandoffId, SubgraphId};

    #[test]
    fn test_dfir_metrics_intervals() {
        let sg_id = SubgraphId::from_raw(0);
        let handoff_id = HandoffId::from_raw(0);

        let mut metrics = DfirMetrics::default();
        metrics.subgraphs.insert(
            sg_id,
            SubgraphMetrics {
                total_run_count: Cell::new(5),
                total_poll_count: Cell::new(10),
                total_idle_count: Cell::new(2),
                total_poll_duration: Cell::new(Duration::from_millis(500)),
                total_idle_duration: Cell::new(Duration::from_millis(200)),
            },
        );
        metrics.handoffs.insert(
            handoff_id,
            HandoffMetrics {
                curr_items_count: Cell::new(3),
                total_items_count: Cell::new(100),
            },
        );
        let metrics = Rc::new(metrics);

        let mut intervals = DfirMetricsIntervals {
            curr: Rc::clone(&metrics),
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
