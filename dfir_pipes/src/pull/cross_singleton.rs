use core::borrow::BorrowMut;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};
use crate::{Context, Toggle};

pin_project! {
    /// Pull combinator that crosses each item from `item_pull` with a singleton value from `singleton_pull`.
    ///
    /// The singleton value is obtained from the first item of `singleton_pull` and cached.
    /// All subsequent items from `item_pull` are paired with this cached singleton value.
    ///
    /// If `singleton_pull` ends before yielding any items, the entire combinator ends immediately.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct CrossSingleton<ItemPull, SinglePull, SingleState> {
        #[pin]
        item_pull: ItemPull,
        #[pin]
        singleton_pull: SinglePull,

        singleton_state: SingleState,
    }
}

impl<ItemPull, SinglePull, SingleState> CrossSingleton<ItemPull, SinglePull, SingleState>
where
    Self: Pull,
{
    pub(crate) const fn new(
        item_pull: ItemPull,
        singleton_pull: SinglePull,
        singleton_state: SingleState,
    ) -> Self {
        Self {
            item_pull,
            singleton_pull,
            singleton_state,
        }
    }
}

impl<ItemPull, SinglePull, SingleState> Pull for CrossSingleton<ItemPull, SinglePull, SingleState>
where
    ItemPull: Pull,
    SinglePull: Pull,
    SinglePull::Item: Clone,
    SingleState: BorrowMut<Option<SinglePull::Item>>,
{
    type Ctx<'ctx> = <ItemPull::Ctx<'ctx> as Context<'ctx>>::Merged<SinglePull::Ctx<'ctx>>;

    type Item = (ItemPull::Item, SinglePull::Item);
    type Meta = ItemPull::Meta;
    type CanPend = <ItemPull::CanPend as Toggle>::Or<SinglePull::CanPend>;
    type CanEnd = <ItemPull::CanEnd as Toggle>::And<SinglePull::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();

        // Set the singleton state only if it is not already set.
        // This short-circuits the `SinglePull` side to the first item only.
        let singleton_state = this.singleton_state.borrow_mut();
        let singleton = match singleton_state {
            Some(singleton) => singleton,
            None => {
                match this
                    .singleton_pull
                    .pull(<ItemPull::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
                {
                    PullStep::Ready(item, _meta) => singleton_state.insert(item),
                    PullStep::Pending(_) => {
                        return PullStep::pending();
                    }
                    PullStep::Ended(_) => {
                        // If `singleton_pull` returns EOS, we return EOS, no fused requirement.
                        // This short-circuits the `ItemPull` side, dropping them.
                        return PullStep::ended();
                    }
                }
            }
        };

        // Stream any items.
        match this
            .item_pull
            .pull(<ItemPull::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
        {
            PullStep::Ready(item, meta) => {
                // TODO(mingwei): use meta of singleton too
                PullStep::Ready((item, singleton.clone()), meta)
            }
            PullStep::Pending(_) => PullStep::pending(),
            // If `item_pull` returns EOS, we return EOS, no fused requirement.
            PullStep::Ended(_) => PullStep::ended(),
        }
    }
}

impl<ItemPull, SinglePull, SingleState> FusedPull
    for CrossSingleton<ItemPull, SinglePull, SingleState>
where
    ItemPull: FusedPull,
    SinglePull: FusedPull,
    SinglePull::Item: Clone,
    SingleState: BorrowMut<Option<SinglePull::Item>>,
{
}
