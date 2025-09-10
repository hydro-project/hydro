use super::HandoffTag;
use super::context::Context;
use super::graph::HandoffData;
use crate::util::slot_vec::SlotVec;

/// Represents a compiled subgraph. Used internally by [Dataflow] to erase the input/output [Handoff] types.
pub(crate) trait Subgraph<'a> {
    fn run(
        &mut self,
        context: &mut Context,
        handoffs: &mut SlotVec<HandoffTag, HandoffData>,
    ) -> Box<dyn 'a + Future<Output = ()>>;
}

impl<'a, Func, Fut> Subgraph<'a> for Func
where
    Func: FnMut(&mut Context, &mut SlotVec<HandoffTag, HandoffData>) -> Fut,
    Fut: 'a + Future<Output = ()>,
{
    fn run(
        &mut self,
        context: &mut Context,
        handoffs: &mut SlotVec<HandoffTag, HandoffData>,
    ) -> Box<dyn 'a + Future<Output = ()>> {
        Box::new((self)(context, handoffs))
    }
}
