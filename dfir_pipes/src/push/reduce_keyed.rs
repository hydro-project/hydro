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
            *this.flush_idx = 1; // mark as initialized
        }
        while !this.flush_items.is_empty() {
            ready!(this.next.as_mut().poll_ready(ctx));
            let item = this.flush_items.pop().unwrap();
            this.next.as_mut().start_send(item, ());
        }
        let step = this.next.poll_finalize(ctx);
        if step.is_done() {
            *this.flush_idx = 0;
        }
        step
    }

    fn size_hint(self: Pin<&mut Self>, _hint: (usize, Option<usize>)) {}
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use std::collections::HashMap;
    use std::pin::Pin;

    use super::ReduceKeyed;
    use crate::Yes;
    use crate::push::test_utils::TestPush;
    use crate::push::{Push, PushStep};

    #[test]
    fn reduce_keyed_emits_on_finalize() {
        let mut map = HashMap::new();
        let mut tp = TestPush::no_pend();
        let mut rk = ReduceKeyed::new(&mut map, |acc: &mut i32, v| *acc += v, &mut tp);
        let mut rk = Pin::new(&mut rk);
        rk.as_mut().poll_ready(&mut ());
        rk.as_mut().start_send((1, 10), ());
        rk.as_mut().poll_ready(&mut ());
        rk.as_mut().start_send((1, 20), ());
        rk.as_mut().poll_ready(&mut ());
        rk.as_mut().start_send((2, 30), ());
        rk.as_mut().poll_finalize(&mut ());
        let mut items = tp.items();
        items.sort();
        assert_eq!(items, vec![(1, 30), (2, 30)]);
    }

    #[test]
    fn reduce_keyed_empty_input() {
        let mut map: HashMap<i32, i32> = HashMap::new();
        let mut tp = TestPush::no_pend();
        let mut rk = ReduceKeyed::new(&mut map, |acc: &mut i32, v| *acc += v, &mut tp);
        let mut rk = Pin::new(&mut rk);
        rk.as_mut().poll_finalize(&mut ());
        assert!(tp.items().is_empty());
    }

    #[test]
    fn reduce_keyed_first_value_is_initial() {
        let mut map = HashMap::new();
        let mut tp = TestPush::no_pend();
        let mut rk = ReduceKeyed::new(&mut map, |acc: &mut i32, v| *acc += v, &mut tp);
        let mut rk = Pin::new(&mut rk);
        rk.as_mut().start_send((1, 42), ());
        rk.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![(1, 42)]);
    }

    #[test]
    fn reduce_keyed_resumes_after_pending() {
        let mut map = HashMap::new();
        let mut tp: TestPush<(i32, i32), Yes, true> =
            TestPush::new_fused([PushStep::Done, PushStep::pending(), PushStep::Done], []);
        let mut rk = ReduceKeyed::new(&mut map, |acc: &mut i32, v| *acc += v, &mut tp);
        let mut rk = Pin::new(&mut rk);
        rk.as_mut().start_send((1, 10), ());
        rk.as_mut().start_send((2, 20), ());
        // First finalize: sends one item, then Pending
        let step = rk.as_mut().poll_finalize(&mut ());
        assert!(step.is_pending());
        // Second finalize: resumes and sends remaining
        let step = rk.as_mut().poll_finalize(&mut ());
        assert!(step.is_done());
        let mut items = tp.items();
        items.sort();
        assert_eq!(items, vec![(1, 10), (2, 20)]);
    }
}
