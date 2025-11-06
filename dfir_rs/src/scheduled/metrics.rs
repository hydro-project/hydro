//! Runtime metrics for DFIR.

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

/// A snapshot of DFIR runtime metrics accumulated since runtime creation.
#[derive(Clone)]
pub struct DfirMetrics {
    pub(super) state: Rc<DfirMetricsState>,
}

impl DfirMetrics {
    /// Returns an iterator over all subgraph IDs.
    pub fn subgraph_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = SubgraphId> + FusedIterator + Clone {
        self.state.subgraph_metrics.keys()
    }

    /// Gets the metrics for a particular subgraph.
    pub fn subgraph_metrics(&self, sg_id: SubgraphId) -> &SubgraphMetrics {
        &self.state.subgraph_metrics[sg_id]
    }

    /// Returns an iterator over all handoff IDs.
    pub fn handoff_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = HandoffId> + FusedIterator + Clone {
        self.state.handoff_metrics.keys()
    }

    /// Gets the metrics for a particular handoff.
    pub fn handoff_metrics(&self, handoff_id: HandoffId) -> &HandoffMetrics {
        &self.state.handoff_metrics[handoff_id]
    }

    /// Subtracts `prev` from `self` to create runtime metrics across a span of time.
    pub fn delta(&self, prev: &Self) -> DfirMetricsDelta {
        DfirMetricsDelta {
            curr: self.state.clone(),
            prev: prev.state.clone(),
        }
    }
}

/// DFIR runtime metrics across a span of time.
pub struct DfirMetricsDelta {
    curr: Rc<DfirMetricsState>,
    prev: Rc<DfirMetricsState>,
}
impl DfirMetricsDelta {
    /// Returns an iterator over all subgraph IDs.
    pub fn subgraph_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = SubgraphId> + FusedIterator + Clone {
        self.curr.subgraph_metrics.keys()
    }

    /// Gets the metrics for a particular subgraph.
    pub fn subgraph_metrics(&self, sg_id: SubgraphId) -> SubgraphMetrics {
        let curr = &self.curr.subgraph_metrics[sg_id];
        let prev = &self.prev.subgraph_metrics[sg_id];
        SubgraphMetrics {
            total_run_count: curr.total_run_count - prev.total_run_count,
            total_poll_duration: curr.total_poll_duration - prev.total_poll_duration,
            total_poll_count: curr.total_poll_count - prev.total_poll_count,
            total_idle_duration: curr.total_idle_duration - prev.total_idle_duration,
            total_idle_count: curr.total_idle_count - prev.total_idle_count,
        }
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
        let prev = &self.prev.handoff_metrics[handoff_id];
        HandoffMetrics {
            curr_items_count: curr.curr_items_count,
            total_items_count: curr.total_items_count - prev.total_items_count,
        }
    }
}

/// Per-handoff metrics.
#[derive(Default, Debug, Clone)]
#[non_exhaustive] // May add more metrics later.
pub struct HandoffMetrics {
    /// Current items in the handoff.
    pub curr_items_count: usize,
    /// Total items read out of this handoff.
    pub total_items_count: usize,
}

/// Per-subgraph metrics.
#[derive(Default, Debug, Clone)]
#[non_exhaustive] // May add more metrics later.
pub struct SubgraphMetrics {
    /// Number of times the subgraph has run.
    pub total_run_count: usize,
    /// Total time elapsed during polling (when the subgraph is actively doing work).
    pub total_poll_duration: Duration,
    /// Number of times the subgraph has been polled.
    pub total_poll_count: usize,
    /// Total time elapsed during idle (when the subgraph has yielded and is waiting for async events).
    pub total_idle_duration: Duration,
    /// Number of times the subgraph has been idle.
    pub total_idle_count: usize,
}

pin_project! {
    /// Helper struct which instruments a future to track polling times.
    pub(crate) struct InstrumentSubgraph<'a, Fut> {
        #[pin]
        future: Fut,
        idle_start: Option<Instant>,
        metrics: &'a mut SubgraphMetrics,
    }
}

impl<'a, Fut> InstrumentSubgraph<'a, Fut> {
    pub(crate) fn new(future: Fut, metrics: &'a mut SubgraphMetrics) -> Self {
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
            this.metrics.total_idle_duration += idle_start.elapsed();
            this.metrics.total_idle_count += 1;
        }

        let poll_start = Instant::now();
        let out = this.future.poll(cx);
        this.idle_start.replace(Instant::now());

        this.metrics.total_poll_duration += poll_start.elapsed();
        this.metrics.total_poll_count += 1;

        out
    }
}
