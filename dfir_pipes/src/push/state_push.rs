//! [`StatePush`] push combinator for lattice state operators.
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::No;
use crate::push::{Push, PushStep};

pin_project! {
    /// Push combinator that merges items into state, forwarding changed items
    /// to `items_push` and emitting the accumulated state to `state_push` on finalize.
    ///
    /// For each item, `mapfn` maps it and `mergefn` merges the mapped value into `state_ref`.
    /// If the merge returns `true` (indicating a change), the original item is forwarded to
    /// `items_push`. On finalize, a clone of the accumulated state is sent to `state_push`.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct StatePush<'a, Item, MappingFn, MergeFn, ItemsPsh, StatePsh, Lat> {
        #[pin]
        items_push: ItemsPsh,
        #[pin]
        state_push: StatePsh,
        mapfn: MappingFn,
        mergefn: MergeFn,
        state_ref: &'a mut Lat,
        _phantom: ::core::marker::PhantomData<fn(Item)>,
    }
}

impl<'a, Item, MappingFn, MappedItem, MergeFn, ItemsPsh, StatePsh, Lat> Push<Item, ()>
    for StatePush<'a, Item, MappingFn, MergeFn, ItemsPsh, StatePsh, Lat>
where
    Item: 'a + Clone,
    MappingFn: 'a + Fn(Item) -> MappedItem,
    MergeFn: 'a + Fn(&mut Lat, MappedItem) -> bool,
    ItemsPsh: 'a + Push<Item, ()>,
    StatePsh: 'a + Push<Lat, ()>,
    Lat: 'a + 'static + Clone,
{
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<No> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.project();
        let changed = (this.mergefn)(this.state_ref, (this.mapfn)(item.clone()));
        if changed {
            this.items_push.start_send(item, ());
        }
    }

    fn poll_finalize(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<No> {
        let this = self.project();
        this.state_push.start_send(this.state_ref.clone(), ());
        PushStep::Done
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
}

/// Creates a [`StatePush`] that merges items into state.
///
/// For each item, `mapfn` maps it and `mergefn` merges the result into `state_ref`,
/// returning `true` if the state changed. Changed items are forwarded to `items_push`.
/// On finalize, the accumulated state is emitted to `state_push`.
pub fn state_push<'a, Item, MappingFn, MappedItem, MergeFn, ItemsPsh, StatePsh, Lat>(
    items_push: ItemsPsh,
    state_push: StatePsh,
    mapfn: MappingFn,
    mergefn: MergeFn,
    state_ref: &'a mut Lat,
) -> StatePush<'a, Item, MappingFn, MergeFn, ItemsPsh, StatePsh, Lat>
where
    Item: 'a + Clone,
    MappingFn: 'a + Fn(Item) -> MappedItem,
    MergeFn: 'a + Fn(&mut Lat, MappedItem) -> bool,
    ItemsPsh: 'a + Push<Item, ()>,
    StatePsh: 'a + Push<Lat, ()>,
    Lat: 'a + 'static + Clone,
{
    StatePush {
        items_push,
        state_push,
        mapfn,
        mergefn,
        state_ref,
        _phantom: ::core::marker::PhantomData,
    }
}
