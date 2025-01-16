use super::context::Context;
use super::graph::HandoffData;
use super::HandoffTag;
use crate::util::slot_vec::SlotVec;

/// Represents a compiled subgraph. Used internally by [Dataflow] to erase the input/output [Handoff] types.
pub(crate) trait Subgraph {
    // TODO: pass in some scheduling info?
    fn run(&mut self, context: &mut Context, handoffs: &mut SlotVec<HandoffTag, HandoffData>);
}
impl<F> Subgraph for F
where
    F: FnMut(&mut Context, &mut SlotVec<HandoffTag, HandoffData>),
{
    fn run(&mut self, context: &mut Context, handoffs: &mut SlotVec<HandoffTag, HandoffData>) {
        (self)(context, handoffs);
    }
}
