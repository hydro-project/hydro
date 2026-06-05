//! [`StatePush`] push combinator for lattice state operators.
use core::pin::Pin;

use lattices::Merge;
use pin_project_lite::pin_project;

use super::ready_both;
use crate::push::{Push, PushStep};
use crate::{Context, Toggle};

pin_project! {
    /// Push combinator that merges items into state, forwarding changed items
    /// to `items_push` and emitting the accumulated state to `state_push` on finalize.
    ///
    /// For each item, `map_fn` maps it and [`Lat::merge`] merges the mapped value into `state_ref`.
    /// If the merge returns `true` (indicating a change), the original item is forwarded to
    /// `items_push`. On finalize, a clone of the accumulated state is sent to `state_push`.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct StatePush<'a, Item, MappingFn, ItemsPsh, StatePsh, Lat> {
        #[pin]
        items_push: ItemsPsh,
        #[pin]
        state_push: StatePsh,
        map_fn: MappingFn,
        state_ref: &'a mut Lat,
        _phantom: ::core::marker::PhantomData<fn(Item)>,
    }
}

impl<'a, Item, MappingFn, MappedItem, ItemsPsh, StatePsh, Lat> Push<Item, ()>
    for StatePush<'a, Item, MappingFn, ItemsPsh, StatePsh, Lat>
where
    Item: Clone,
    MappingFn: Fn(Item) -> MappedItem,
    ItemsPsh: Push<Item, ()>,
    StatePsh: Push<Lat, ()>,
    Lat: Merge<MappedItem> + Clone,
{
    type Ctx<'ctx> = <ItemsPsh::Ctx<'ctx> as Context<'ctx>>::Merged<StatePsh::Ctx<'ctx>>;
    type CanPend = <ItemsPsh::CanPend as Toggle>::Or<StatePsh::CanPend>;

    fn poll_ready(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let this = self.project();
        ready_both!(
            this.items_push
                .poll_ready(<ItemsPsh::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            this.state_push
                .poll_ready(<ItemsPsh::Ctx<'_> as Context<'_>>::unmerge_other(ctx)),
        );
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.project();
        let changed = Lat::merge(this.state_ref, (this.map_fn)(item.clone()));
        if changed {
            this.items_push.start_send(item, ());
        }
    }

    fn poll_finalize(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        this.state_push
            .as_mut()
            .start_send(this.state_ref.clone(), ());
        ready_both!(
            this.items_push
                .poll_finalize(<ItemsPsh::Ctx<'_> as Context<'_>>::unmerge_self(ctx)),
            this.state_push
                .poll_finalize(<ItemsPsh::Ctx<'_> as Context<'_>>::unmerge_other(ctx)),
        );
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
}

/// Creates a [`StatePush`] that merges items into state.
///
/// For each item, `map_fn` maps it and [`Lat::merge`](Merge::merge) merges the result
/// into `state_ref`, returning `true` if the state changed. Changed items are forwarded
/// to `items_push`. On finalize, the accumulated state is emitted to `state_push`.
pub fn state_push<Item, MappingFn, MappedItem, ItemsPsh, StatePsh, Lat>(
    items_push: ItemsPsh,
    state_push: StatePsh,
    map_fn: MappingFn,
    state_ref: &mut Lat,
) -> StatePush<'_, Item, MappingFn, ItemsPsh, StatePsh, Lat>
where
    Item: Clone,
    MappingFn: Fn(Item) -> MappedItem,
    ItemsPsh: Push<Item, ()>,
    StatePsh: Push<Lat, ()>,
    Lat: Merge<MappedItem> + Clone,
{
    StatePush {
        items_push,
        state_push,
        map_fn,
        state_ref,
        _phantom: ::core::marker::PhantomData,
    }
}
