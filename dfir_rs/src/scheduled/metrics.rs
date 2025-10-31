//! Runtime metrics for DFIR.

use std::iter::FusedIterator;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use pin_project_lite::pin_project;

use super::graph::Dfir;
use super::{HandoffId, SubgraphId};

/// A view into the runtime metrics of a DFIR instance.
pub struct DfirMetrics<'dfir, 'sg> {
    pub(super) dfir: &'dfir Dfir<'sg>,
}
impl DfirMetrics<'_, '_> {
    /// Returns an iterator over all subgraph IDs.
    pub fn subgraph_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = SubgraphId> + ExactSizeIterator + FusedIterator + Clone
    {
        self.dfir.subgraphs.keys()
    }

    /// Gets the metrics for a particular subgraph.
    pub fn subgraph_metrics(&self, sg_id: SubgraphId) -> &SubgraphMetrics {
        &self.dfir.subgraphs[sg_id].metrics
    }

    /// Returns an iterator over all handoff IDs.
    pub fn handoff_ids(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = HandoffId> + ExactSizeIterator + FusedIterator + Clone
    {
        self.dfir.handoffs.keys()
    }

    /// Gets the metrics for a particular handoff.
    pub fn handoff_metrics(&self, handoff_id: HandoffId) -> &HandoffMetrics {
        &self.dfir.handoffs[handoff_id].metrics
    }
}

/// Per-handoff metrics.
#[derive(Default)]
#[non_exhaustive] // May add more metrics later.
pub struct HandoffMetrics {
    /// Total items read out of this handoff.
    pub total_items_count: usize,
}

/// Per-subgraph metrics.
#[derive(Default)]
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
