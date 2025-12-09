//! Module for [`Reactor`].

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::SendError;

use super::SubgraphId;

/// A handle into a specific [super::graph::Dfir] instance for triggering
/// subgraphs to run, possibly from another thread.
///
/// Reactor events are considered to be external events.
#[derive(Clone)]
pub struct Reactor {
    event_queue_send: UnboundedSender<(SubgraphId, bool)>,
}
impl Reactor {
    pub(crate) fn new(event_queue_send: UnboundedSender<(SubgraphId, bool)>) -> Self {
        Self { event_queue_send }
    }

    /// Trigger a subgraph as an external event.
    pub fn trigger(&self, sg_id: SubgraphId) -> Result<(), SendError<(SubgraphId, bool)>> {
        self.event_queue_send.send((sg_id, true))
    }

    /// Convert this `Reactor` into a [`std::task::Waker`] for use with async runtimes.
    pub fn into_waker(self, sg_id: SubgraphId) -> std::task::Waker {
        use std::sync::Arc;
        use std::task::Wake;

        struct ReactorWaker {
            reactor: Reactor,
            sg_id: SubgraphId,
        }
        impl Wake for ReactorWaker {
            fn wake(self: Arc<Self>) {
                self.wake_by_ref();
            }

            fn wake_by_ref(self: &Arc<Self>) {
                let _recv_closed_error = self.reactor.trigger(self.sg_id);
            }
        }

        let reactor_waker = ReactorWaker {
            reactor: self,
            sg_id,
        };
        std::task::Waker::from(Arc::new(reactor_waker))
    }
}
