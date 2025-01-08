use super::context::Context;
use super::graph::HandoffData;

/// Represents a compiled subgraph. Used internally by [Dataflow] to erase the input/output [Handoff] types.
pub(crate) trait Subgraph {
    // TODO: pass in some scheduling info?
    fn run(&mut self, context: &mut Context, handoffs: &mut Vec<HandoffData>);
}
impl<F> Subgraph for F
where
    F: FnMut(&mut Context, &mut Vec<HandoffData>),
{
    fn run(&mut self, context: &mut Context, handoffs: &mut Vec<HandoffData>) {
        (self)(context, handoffs);
    }
}
