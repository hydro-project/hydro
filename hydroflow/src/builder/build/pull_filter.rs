use super::{PullBuild, PullBuildBase};

use crate::scheduled::{context::Context, handoff::handoff_list::PortList, port::RECV};

pub struct FilterPullBuild<Prev, Func>
where
    Prev: PullBuild,
{
    prev: Prev,
    func: Func,
}
impl<Prev, Func> FilterPullBuild<Prev, Func>
where
    Prev: PullBuild,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    pub fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}

#[allow(type_alias_bounds)]
type PullBuildImpl<'slf, 'ctx, Prev, Func>
where
    Prev: PullBuild,
= std::iter::Filter<Prev::Build<'slf, 'ctx>, impl FnMut(&Prev::ItemOut) -> bool>;

impl<Prev, Func> PullBuildBase for FilterPullBuild<Prev, Func>
where
    Prev: PullBuild,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    type ItemOut = Prev::ItemOut;
    type Build<'slf, 'ctx> = PullBuildImpl<'slf, 'ctx, Prev, Func>;
}

impl<Prev, Func> PullBuild for FilterPullBuild<Prev, Func>
where
    Prev: PullBuild,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    type InputHandoffs = Prev::InputHandoffs;

    fn build<'slf, 'ctx>(
        &'slf mut self,
        context: &'ctx Context<'ctx>,
        handoffs: <Self::InputHandoffs as PortList<RECV>>::Ctx<'ctx>,
    ) -> Self::Build<'slf, 'ctx> {
        self.prev
            .build(context, handoffs)
            .filter(|x| (self.func)(x))
    }
}
