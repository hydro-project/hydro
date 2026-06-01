//! [`ReduceKeyed`] push combinator.
use core::hash::{BuildHasher, Hash};
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

extern crate alloc;
use alloc::vec::Vec;

pin_project! {
    /// Push combinator that reduces items by key into a hashmap, then emits all
    /// (key, value) pairs downstream on flush. The first value for each key
    /// becomes the initial accumulator.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct ReduceKeyed<MapRef, ReduceFn, Next, K, V> {
        #[pin]
        next: Next,
        map: MapRef,
        reduce_fn: ReduceFn,
        flush_items: Vec<(K, V)>,
        flush_idx: usize,
    }
}

impl<MapRef, ReduceFn, Next, K, V> ReduceKeyed<MapRef, ReduceFn, Next, K, V> {
    /// Creates a new `ReduceKeyed` push combinator.
    pub const fn new(map: MapRef, reduce_fn: ReduceFn, next: Next) -> Self {
        Self {
            next,
            map,
            reduce_fn,
            flush_items: Vec::new(),
            flush_idx: 0,
        }
    }
}

// TODO(mingwei): support arbitrary metadata.
impl<K, V, S, ReduceFn, Next> Push<(K, V), ()>
    for ReduceKeyed<&mut std::collections::HashMap<K, V, S>, ReduceFn, Next, K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
    S: BuildHasher,
    ReduceFn: FnMut(&mut V, V),
    Next: Push<(K, V), ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: (K, V), _meta: ()) {
        let this = self.project();
        match this.map.entry(item.0) {
            std::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(item.1);
            }
            std::collections::hash_map::Entry::Occupied(mut occupied) => {
                (this.reduce_fn)(occupied.get_mut(), item.1);
            }
        }
    }

    fn poll_finalize(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        let mut this = self.project();
        if this.flush_items.is_empty() && *this.flush_idx == 0 {
            #[expect(
                clippy::disallowed_methods,
                reason = "collected into a Vec; key order is irrelevant"
            )]
            {
                this.flush_items
                    .extend(this.map.iter().map(|(k, v)| (k.clone(), v.clone())));
            }
        }
        while *this.flush_idx < this.flush_items.len() {
            ready!(this.next.as_mut().poll_ready(ctx));
            let item = this.flush_items[*this.flush_idx].clone();
            this.next.as_mut().start_send(item, ());
            *this.flush_idx += 1;
        }
        let step = this.next.poll_finalize(ctx);
        if step.is_done() {
            this.flush_items.clear();
            *this.flush_idx = 0;
        }
        step
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
}
