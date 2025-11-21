use std::cell::RefCell;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use futures::stream::FusedStream;
use lattices::{LatticeBimorphism, Merge};
use pin_project_lite::pin_project;

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct LatticeBimorphismStream<'a, Func, LhsStream, RhsStream, LhsState, RhsState, Output> {
        #[pin]
        lhs_stream: LhsStream,
        #[pin]
        rhs_stream: RhsStream,

        func: Func,

        lhs_state: &'a RefCell<LhsState>,
        rhs_state: &'a RefCell<RhsState>,

        output: Option<Output>,
    }
}

impl<'a, Func, LhsStream, RhsStream, LhsState, RhsState, Output>
    LatticeBimorphismStream<'a, Func, LhsStream, RhsStream, LhsState, RhsState, Output>
where
    Func: 'a
        + LatticeBimorphism<LhsState, RhsStream::Item, Output = Output>
        + LatticeBimorphism<LhsStream::Item, RhsState, Output = Output>,
    LhsStream: 'a + FusedStream,
    RhsStream: 'a + FusedStream,
    LhsState: 'static + Clone,
    RhsState: 'static + Clone,
    Output: Merge<Output>,
{
    pub fn new(
        lhs_stream: LhsStream,
        rhs_stream: RhsStream,
        func: Func,
        lhs_state: &'a RefCell<LhsState>,
        rhs_state: &'a RefCell<RhsState>,
    ) -> Self {
        Self {
            lhs_stream,
            rhs_stream,
            func,
            lhs_state,
            rhs_state,
            output: None,
        }
    }
}

impl<'a, Func, LhsStream, RhsStream, LhsState, RhsState, Output> Stream
    for LatticeBimorphismStream<'a, Func, LhsStream, RhsStream, LhsState, RhsState, Output>
where
    Func: 'a
        + LatticeBimorphism<LhsState, RhsStream::Item, Output = Output>
        + LatticeBimorphism<LhsStream::Item, RhsState, Output = Output>,
    LhsStream: 'a + FusedStream,
    RhsStream: 'a + FusedStream,
    LhsState: 'static + Clone,
    RhsState: 'static + Clone,
    Output: Merge<Output>,
{
    type Item = Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let lhs_poll = this.lhs_stream.as_mut().poll_next(cx);
            let lhs_pending = lhs_poll.is_pending();
            let mut live = false;

            if let Poll::Ready(Some(lhs_item)) = lhs_poll {
                live = true;
                let delta = this.func.call(lhs_item, this.rhs_state.borrow().clone());
                if let Some(output) = this.output.as_mut() {
                    output.merge(delta);
                } else {
                    this.output.replace(delta);
                }
            }

            let rhs_poll = this.rhs_stream.as_mut().poll_next(cx);
            let rhs_pending = rhs_poll.is_pending();
            if let Poll::Ready(Some(rhs_item)) = rhs_poll {
                live = true;
                let delta = this.func.call(this.lhs_state.borrow().clone(), rhs_item);
                if let Some(output) = this.output.as_mut() {
                    output.merge(delta);
                } else {
                    this.output.replace(delta);
                }
            }

            if rhs_pending && lhs_pending {
                return Poll::Pending;
            }

            if !live && !rhs_pending && !lhs_pending {
                return Poll::Ready(this.output.take());
            }
            // Both streams may continue to be polled EOS (`None`) on subsequent loops or calls, so they must be fused.
        }
    }
}
