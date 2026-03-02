use std::pin::Pin;

use dfir_pipes::{Context, Pull, Step, Toggle};
use pin_project_lite::pin_project;

pin_project! {
    /// Stream combinator that crosses each item from `item_pull` with a singleton value from `singleton_pull`.
    pub struct CrossSingletonPull<'a, ItemPull, SinglePull, Item> {
        #[pin]
        item_pull: ItemPull,
        #[pin]
        singleton_pull: SinglePull,

        singleton_state: &'a mut Option<Item>,
    }
}

impl<'a, ItemPull, SinglePull> CrossSingletonPull<'a, ItemPull, SinglePull, SinglePull::Item>
where
    ItemPull: Pull,
    SinglePull: Pull,
    SinglePull::Item: Clone,
{
    /// Creates a new `CrossSingletonPull` stream combinator.
    pub fn new(
        item_pull: ItemPull,
        singleton_pull: SinglePull,
        singleton_state: &'a mut Option<SinglePull::Item>,
    ) -> Self {
        Self {
            item_pull,
            singleton_pull,
            singleton_state,
        }
    }
}

impl<'a, ItemPull, SinglePull> Pull
    for CrossSingletonPull<'a, ItemPull, SinglePull, SinglePull::Item>
where
    ItemPull: Pull,
    SinglePull: Pull,
    SinglePull::Item: Clone,
{
    type Ctx<'ctx> = <ItemPull::Ctx<'ctx> as Context<'ctx>>::Merged<SinglePull::Ctx<'ctx>>;

    type Item = (ItemPull::Item, SinglePull::Item);
    type Meta = ItemPull::Meta;
    type CanPend = <ItemPull::CanPend as Toggle>::Or<SinglePull::CanPend>;
    type CanEnd = <ItemPull::CanEnd as Toggle>::And<SinglePull::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();

        // Set the singleton state only if it is not already set.
        // This short-circuits the `SinglePull` side to the first item only.
        let singleton = match this.singleton_state {
            Some(singleton) => singleton,
            None => {
                match this
                    .singleton_pull
                    .pull(<ItemPull::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
                {
                    Step::Ready(item, _meta) => {
                        this.singleton_state.insert(item)
                    }
                    Step::Pending(can_pend) => {
                        return Step::Pending(Toggle::convert_from(can_pend));
                    }
                    Step::Ended(can_end) => {
                        // If `singleton_pull` returns EOS, we return EOS, no fused requirement.
                        // This short-circuits the `ItemPull` side, dropping them.
                        return Step::Ended(Toggle::convert_from(can_end));
                    }
                }
            }
        };

        // Stream any items.
        match this.item_pull.pull(<ItemPull::Ctx<'_> as Context<'_>>::unmerge_self(ctx)) {
            Step::Ready(item, meta) => {
                // TODO(mingwei): use meta of singleton too
                Step::Ready((item, singleton.clone()), meta)
            },
            Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
            // If `item_pull` returns EOS, we return EOS, no fused requirement.
            Step::Ended(can_end) => Step::Ended(Toggle::convert_from(can_end)),
        }
    }
}
