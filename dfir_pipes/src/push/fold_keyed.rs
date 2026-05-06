//! [`FoldKeyed`] push combinator.
use core::hash::{BuildHasher, Hash};
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

extern crate alloc;
use alloc::vec::Vec;

pin_project! {
    /// Push combinator that folds items by key into a hashmap, then emits all
    /// (key, value) pairs downstream on flush.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct FoldKeyed<MapRef, InitFn, CombFn, Next, K, V> {
        #[pin]
        next: Next,
        map: MapRef,
        init_fn: InitFn,
        comb_fn: CombFn,
        flush_items: Vec<(K, V)>,
        flush_idx: usize,
    }
}

impl<MapRef, InitFn, CombFn, Next, K, V> FoldKeyed<MapRef, InitFn, CombFn, Next, K, V> {
    /// Creates a new `FoldKeyed` push combinator.
    pub const fn new(map: MapRef, init_fn: InitFn, comb_fn: CombFn, next: Next) -> Self {
        Self {
            next,
            map,
            init_fn,
            comb_fn,
            flush_items: Vec::new(),
            flush_idx: 0,
        }
    }
}

// Impl for `&mut HashMap<K, V, S>` (works with any hasher including FxHashMap).
// TODO(mingwei): support arbitrary metadata.
impl<K, V, S, InitFn, CombFn, Next> Push<(K, V), ()>
    for FoldKeyed<&mut std::collections::HashMap<K, V, S>, InitFn, CombFn, Next, K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
    S: BuildHasher,
    InitFn: FnMut() -> V,
    CombFn: FnMut(&mut V, V),
    Next: Push<(K, V), ()>,
{
    type Ctx<'ctx> = Next::Ctx<'ctx>;

    type CanPend = Next::CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: (K, V), _meta: ()) {
        let this = self.project();
        let entry = this.map.entry(item.0).or_insert_with(|| (this.init_fn)());
        (this.comb_fn)(entry, item.1);
    }

    fn poll_flush(self: Pin<&mut Self>, ctx: &mut Self::Ctx<'_>) -> PushStep<Self::CanPend> {
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
        this.flush_items.clear();
        *this.flush_idx = 0;
        this.next.poll_flush(ctx)
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
}
