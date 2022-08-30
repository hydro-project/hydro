use super::PushBuild;

use crate::scheduled::context::Context;
use crate::scheduled::handoff::handoff_list::{PortList, PortListSplit};
use crate::scheduled::port::SEND;
use crate::scheduled::type_list::Extend;
use pusherator::partition::Partition;

pub struct PartitionPushBuild<NextA, NextB, Func>
where
    Func: FnMut(&Context, &NextA::ItemIn) -> bool,
    NextA: PushBuild,
    NextB: PushBuild<ItemIn = NextA::ItemIn>,

    NextA::OutputHandoffs: Extend<NextB::OutputHandoffs>,
    <NextA::OutputHandoffs as Extend<NextB::OutputHandoffs>>::Extended:
        PortList<SEND> + PortListSplit<SEND, NextA::OutputHandoffs, Suffix = NextB::OutputHandoffs>,
{
    func: Func,
    next_a: NextA,
    next_b: NextB,
}
impl<Func, NextA, NextB> PartitionPushBuild<NextA, NextB, Func>
where
    Func: FnMut(&Context, &NextA::ItemIn) -> bool,
    NextA: PushBuild,
    NextB: PushBuild<ItemIn = NextA::ItemIn>,

    NextA::OutputHandoffs: Extend<NextB::OutputHandoffs>,
    <NextA::OutputHandoffs as Extend<NextB::OutputHandoffs>>::Extended:
        PortList<SEND> + PortListSplit<SEND, NextA::OutputHandoffs, Suffix = NextB::OutputHandoffs>,
{
    pub fn new(func: Func, next_a: NextA, next_b: NextB) -> Self {
        Self {
            func,
            next_a,
            next_b,
        }
    }
}

#[allow(type_alias_bounds)]
type PushBuildImpl<'slf, 'ctx, NextA, NextB, Func>
where
    Func: 'slf + FnMut(&Context, &NextA::ItemIn) -> bool,
    NextA: PushBuild,
    NextB: PushBuild<ItemIn = NextA::ItemIn>,

    NextA::OutputHandoffs: Extend<NextB::OutputHandoffs>,
    <NextA::OutputHandoffs as Extend<NextB::OutputHandoffs>>::Extended:
        PortList<SEND> + PortListSplit<SEND, NextA::OutputHandoffs, Suffix = NextB::OutputHandoffs>,
= Partition<NextA::Build<'slf, 'ctx>, NextB::Build<'slf, 'ctx>, impl FnMut(&NextA::ItemIn) -> bool>;

impl<NextA, NextB, Func> PushBuild for PartitionPushBuild<NextA, NextB, Func>
where
    Func: FnMut(&Context, &NextA::ItemIn) -> bool,
    NextA: PushBuild,
    NextB: PushBuild<ItemIn = NextA::ItemIn>,

    NextA::OutputHandoffs: Extend<NextB::OutputHandoffs>,
    <NextA::OutputHandoffs as Extend<NextB::OutputHandoffs>>::Extended:
        PortList<SEND> + PortListSplit<SEND, NextA::OutputHandoffs, Suffix = NextB::OutputHandoffs>,
{
    type ItemIn = NextA::ItemIn;
    type Build<'slf, 'ctx> = PushBuildImpl<'slf, 'ctx, NextA, NextB, Func>
    where
        Self: 'slf;

    type OutputHandoffs = <NextA::OutputHandoffs as Extend<NextB::OutputHandoffs>>::Extended;

    fn build<'slf, 'ctx>(
        &'slf mut self,
        context: &'ctx Context,
        input: <Self::OutputHandoffs as PortList<SEND>>::Ctx<'ctx>,
    ) -> Self::Build<'slf, 'ctx> {
        let (input_a, input_b) = <Self::OutputHandoffs as PortListSplit<_, _>>::split_ctx(input);
        let build_a = self.next_a.build(context, input_a);
        let build_b = self.next_b.build(context, input_b);
        Partition::new(|x| (self.func)(context, x), build_a, build_b)
    }
}
