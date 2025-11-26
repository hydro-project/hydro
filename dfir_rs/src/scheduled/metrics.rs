//! Runtime metrics for DFIR.

use std::cell::Cell;
use std::iter::FusedIterator;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use web_time::{Duration, Instant};

use super::{HandoffId, HandoffTag, SubgraphId, SubgraphTag};
use crate::util::slot_vec::SecondarySlotVec;

#[derive(Default, Clone)]
pub(super) struct DfirMetricsState {
    pub(super) subgraph_metrics: SecondarySlotVec<SubgraphTag, SubgraphMetrics>,
    pub(super) handoff_metrics: SecondarySlotVec<HandoffTag, HandoffMetrics>,
}

/// DFIR runtime metrics accumulated across a time, possibly since runtime creation.
#[derive(Clone)]
pub struct DfirMetrics {
    pub(super) curr: Rc<DfirMetricsState>,
    /// `None` for "since creation".
    pub(super) prev: Option<DfirMetricsState>,
}
impl DfirMetrics {
    /// Begins a new metrics collection period, effectively resetting all metrics to zero.
    pub fn reset(&mut self) {
        self.prev = Some(self.curr.as_ref().clone());
    }

    /// Returns an iterator over all subgraph IDs.
    pub fn subgraph_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = SubgraphId> + FusedIterator + Clone {
        self.curr.subgraph_metrics.keys()
    }

    /// Gets the metrics for a particular subgraph.
    pub fn subgraph_metrics(&self, sg_id: SubgraphId) -> SubgraphMetrics {
        let curr = &self.curr.subgraph_metrics[sg_id];
        self.prev
            .as_ref()
            .map(|prev| &prev.subgraph_metrics[sg_id])
            .map(|prev| SubgraphMetrics {
                total_run_count: Cell::new(curr.total_run_count.get() - prev.total_run_count.get()),
                total_poll_duration: Cell::new(
                    curr.total_poll_duration.get() - prev.total_poll_duration.get(),
                ),
                total_poll_count: Cell::new(
                    curr.total_poll_count.get() - prev.total_poll_count.get(),
                ),
                total_idle_duration: Cell::new(
                    curr.total_idle_duration.get() - prev.total_idle_duration.get(),
                ),
                total_idle_count: Cell::new(
                    curr.total_idle_count.get() - prev.total_idle_count.get(),
                ),
            })
            .unwrap_or_else(|| curr.clone())
    }

    /// Returns an iterator over all handoff IDs.
    pub fn handoff_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = HandoffId> + FusedIterator + Clone {
        self.curr.handoff_metrics.keys()
    }

    /// Gets the metrics for a particular handoff.
    pub fn handoff_metrics(&self, handoff_id: HandoffId) -> HandoffMetrics {
        let curr = &self.curr.handoff_metrics[handoff_id];
        self.prev
            .as_ref()
            .map(|prev| &prev.handoff_metrics[handoff_id])
            .map(|prev| HandoffMetrics {
                total_items_count: Cell::new(
                    curr.total_items_count.get() - prev.total_items_count.get(),
                ),
                curr_items_count: curr.curr_items_count.clone(),
            })
            .unwrap_or_else(|| curr.clone())
    }
}

/// Declarative macro to generate metrics structs with Cell-based fields and getter methods.
macro_rules! define_getters {
    (
        $(#[$struct_attr:meta])*
        pub struct $struct_name:ident {
            $(
                $(#[$field_attr:meta])*
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
                $(#[$field_attr])*
                pub fn $field_name(&self) -> $field_type {
                    self.$field_name.get()
                }
            )*
        }
    };
}

define_getters! {
    /// Per-handoff metrics.
    pub struct HandoffMetrics {
        /// Number of items currently in the handoff.
        pub(super) curr_items_count: Cell<usize>,
        /// Total number of items read out of the handoff.
        pub(super) total_items_count: Cell<usize>,
    }
}

define_getters! {
    /// Per-subgraph metrics.
    pub struct SubgraphMetrics {
        /// Number of times the subgraph has run.
        pub(super) total_run_count: Cell<usize>,
        /// Total time elapsed during polling (when the subgraph is actively doing work).
        pub(super) total_poll_duration: Cell<Duration>,
        /// Number of times the subgraph has been polled.
        pub(super) total_poll_count: Cell<usize>,
        /// Total time elapsed during idle (when the subgraph has yielded and is waiting for async events).
        pub(super) total_idle_duration: Cell<Duration>,
        /// Number of times the subgraph has been idle.
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

        if let Some(idle_start) = this.idle_start {
            this.metrics
                .total_idle_duration
                .update(|x| x + idle_start.elapsed());
            this.metrics.total_idle_count.update(|x| x + 1);
        }

        let poll_start = Instant::now();
        let out = this.future.poll(cx);
        this.idle_start.replace(Instant::now());

        this.metrics
            .total_poll_duration
            .update(|x| x + poll_start.elapsed());
        this.metrics.total_poll_count.update(|x| x + 1);

        out
    }
}
