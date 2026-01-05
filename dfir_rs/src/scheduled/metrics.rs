//! Runtime metrics for DFIR.

use std::cell::Cell;
use std::iter::FusedIterator;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use web_time::{Duration, Instant};

#[expect(unused_imports, reason = "used for rustdoc links")]
use super::graph::Dfir;
use super::{HandoffId, HandoffTag, SubgraphId, SubgraphTag};
use crate::util::slot_vec::SecondarySlotVec;

/// Metrics for a [`Dfir`] graph instance.
///
/// Call [`Dfir::metrics`] for referenced-counted continually-updated metrics,
/// or call [`Dfir::metrics_intervals`] for an infinte iterator of metrics (across each interval).
#[derive(Default, Clone)]
pub struct DfirMetrics {
    pub(super) subgraph_metrics: SecondarySlotVec<SubgraphTag, SubgraphMetrics>,
    pub(super) handoff_metrics: SecondarySlotVec<HandoffTag, HandoffMetrics>,
}

impl DfirMetrics {
    /// Returns an iterator over all subgraph IDs.
    pub fn subgraph_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = SubgraphId> + FusedIterator + Clone {
        self.subgraph_metrics.keys()
    }

    /// Gets the metrics for a particular subgraph.
    pub fn subgraph_metrics(&self, sg_id: SubgraphId) -> &SubgraphMetrics {
        &self.subgraph_metrics[sg_id]
    }

    /// Returns an iterator over all handoff IDs.
    pub fn handoff_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = HandoffId> + FusedIterator + Clone {
        self.handoff_metrics.keys()
    }

    /// Gets the metrics for a particular handoff.
    pub fn handoff_metrics(&self, handoff_id: HandoffId) -> &HandoffMetrics {
        &self.handoff_metrics[handoff_id]
    }

    /// Subtracts `other` from self.
    fn diff(&mut self, other: &Self) {
        for (sg_id, prev_sg_metrics) in other.subgraph_metrics.iter() {
            if let Some(curr_sg_metrics) = self.subgraph_metrics.get_mut(sg_id) {
                curr_sg_metrics.diff(prev_sg_metrics);
            }
        }
        for (handoff_id, prev_handoff_metrics) in other.handoff_metrics.iter() {
            if let Some(curr_handoff_metrics) = self.handoff_metrics.get_mut(handoff_id) {
                curr_handoff_metrics.diff(prev_handoff_metrics);
            }
        }
    }
}

/// Created via [`Dfir::metrics_intervals`], see its documentation for details.
#[derive(Clone)]
pub struct DfirMetricsIntervals {
    /// `curr` is continually updating (via shared ownership).
    pub(super) curr: Rc<DfirMetrics>,
    /// `prev` is an unchanging snapshot in time. `None` for "since creation".
    pub(super) prev: Option<DfirMetrics>,
}

impl Iterator for DfirMetricsIntervals {
    type Item = DfirMetrics;

    fn next(&mut self) -> Option<Self::Item> {
        let mut curr = self.curr.as_ref().clone();
        if let Some(prev) = self.prev.replace(curr.clone()) {
            curr.diff(&prev);
        }
        Some(curr)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::MAX, None)
    }

    #[track_caller]
    fn last(self) -> Option<DfirMetrics> {
        panic!("iterator is infinite");
    }

    #[track_caller]
    fn count(self) -> usize {
        panic!("iterator is infinite");
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
        debug_assert!($other.$field.get() < $slf.$field.get());
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
