use super::HandoffTag;
use super::context::Context;
use super::graph::HandoffData;
use crate::util::slot_vec::SlotVec;

/// Represents a compiled subgraph. Used internally by [Dataflow] to erase the input/output [Handoff] types.
pub(crate) trait Subgraph<'a> {
    fn run<'ctx>(
        &'ctx mut self,
        context: &'ctx mut Context,
        handoffs: &'ctx mut SlotVec<HandoffTag, HandoffData>,
    ) -> Box<dyn 'ctx + Future<Output = ()>>;
}

impl<'a, Func> Subgraph<'a> for Func
where
    Func: 'a + AsyncFnMut(&mut Context, &mut SlotVec<HandoffTag, HandoffData>),
{
    fn run<'ctx>(
        &'ctx mut self,
        context: &'ctx mut Context,
        handoffs: &'ctx mut SlotVec<HandoffTag, HandoffData>,
    ) -> Box<dyn 'ctx + Future<Output = ()>> {
        Box::new((self)(context, handoffs))
    }
}
