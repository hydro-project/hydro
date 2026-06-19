//! [`FoldKeyed`] push combinator.
use core::hash::{BuildHasher, Hash};
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::push::{Push, PushStep, ready};

extern crate alloc;
use alloc::vec::Vec;

pin_project! {
    /// Push combinator that folds items by key into a hashmap, then emits all
    /// (key, accumulator) pairs downstream on finalize.
    ///
    /// Input items are `(K, V)`, accumulated into `HashMap<K, Acc>` via
    /// `InitFn: FnMut() -> Acc` and `CombFn: FnMut(&mut Acc, V)`, then
    /// emits `(K, Acc)` downstream.
    #[must_use = "`Push`es do nothing unless items are pushed into them"]
    pub struct FoldKeyed<MapRef, InitFn, CombFn, Next, K, V, Acc> {
        #[pin]
        next: Next,
        map: MapRef,
        init_fn: InitFn,
        comb_fn: CombFn,
        flush_items: Vec<(K, Acc)>,
        flush_idx: usize,
        _phantom: core::marker::PhantomData<fn(V)>,
    }
}

impl<MapRef, InitFn, CombFn, Next, K, V, Acc> FoldKeyed<MapRef, InitFn, CombFn, Next, K, V, Acc> {
    /// Creates a new `FoldKeyed` push combinator.
    pub const fn new(map: MapRef, init_fn: InitFn, comb_fn: CombFn, next: Next) -> Self {
        Self {
            next,
            map,
            init_fn,
            comb_fn,
            flush_items: Vec::new(),
            flush_idx: 0,
            _phantom: core::marker::PhantomData,
        }
    }
}

// Impl for `&mut HashMap<K, Acc, S>` (works with any hasher including FxHashMap).
// TODO(mingwei): support arbitrary metadata.
impl<K, V, Acc, S, InitFn, CombFn, Next> Push<(K, V), ()>
    for FoldKeyed<&mut std::collections::HashMap<K, Acc, S>, InitFn, CombFn, Next, K, V, Acc>
where
    K: Eq + Hash + Clone,
    Acc: Clone,
    S: BuildHasher,
    InitFn: FnMut() -> Acc,
    CombFn: FnMut(&mut Acc, V),
    Next: Push<(K, Acc), ()>,
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
    use alloc::string::String;
    use alloc::vec;
    use std::collections::HashMap;
    use std::pin::Pin;

    use super::FoldKeyed;
    use crate::Yes;
    use crate::push::test_utils::TestPush;
    use crate::push::{Push, PushStep};

    #[test]
    fn fold_keyed_emits_on_finalize() {
        let mut map = HashMap::new();
        let mut tp = TestPush::no_pend();
        let mut fk = FoldKeyed::new(&mut map, || 0i32, |acc: &mut i32, v| *acc += v, &mut tp);
        let mut fk = Pin::new(&mut fk);
        fk.as_mut().poll_ready(&mut ());
        fk.as_mut().start_send((1, 10), ());
        fk.as_mut().poll_ready(&mut ());
        fk.as_mut().start_send((1, 20), ());
        fk.as_mut().poll_ready(&mut ());
        fk.as_mut().start_send((2, 30), ());
        fk.as_mut().poll_finalize(&mut ());
        let mut items = tp.items();
        items.sort();
        assert_eq!(items, vec![(1, 30), (2, 30)]);
    }

    #[test]
    fn fold_keyed_different_acc_type() {
        // Input (K, V) = (i32, &str), Acc = String
        let mut map: HashMap<i32, String> = HashMap::new();
        let mut tp = TestPush::no_pend();
        let mut fk = FoldKeyed::new(
            &mut map,
            String::new,
            |acc: &mut String, v: &str| acc.push_str(v),
            &mut tp,
        );
        let mut fk = Pin::new(&mut fk);
        fk.as_mut().start_send((1, "hello"), ());
        fk.as_mut().start_send((1, " world"), ());
        fk.as_mut().poll_finalize(&mut ());
        assert_eq!(tp.items(), vec![(1, String::from("hello world"))]);
    }

    #[test]
    fn fold_keyed_empty_input() {
        let mut map: HashMap<i32, i32> = HashMap::new();
        let mut tp = TestPush::no_pend();
        let mut fk = FoldKeyed::new(
            &mut map,
            || 0i32,
            |acc: &mut i32, v: i32| *acc += v,
            &mut tp,
        );
        let mut fk = Pin::new(&mut fk);
        fk.as_mut().poll_finalize(&mut ());
        assert!(tp.items().is_empty());
    }

    #[test]
    fn fold_keyed_resumes_after_pending() {
        let mut map = HashMap::new();
        let mut tp: TestPush<(i32, i32), Yes, true> =
            TestPush::new_fused([PushStep::Done, PushStep::pending(), PushStep::Done], []);
        let mut fk = FoldKeyed::new(&mut map, || 0i32, |acc: &mut i32, v| *acc += v, &mut tp);
        let mut fk = Pin::new(&mut fk);
        fk.as_mut().start_send((1, 10), ());
        fk.as_mut().start_send((2, 20), ());
        // First finalize: sends one item, then Pending
        let step = fk.as_mut().poll_finalize(&mut ());
        assert!(step.is_pending());
        // Second finalize: resumes and sends remaining
        let step = fk.as_mut().poll_finalize(&mut ());
        assert!(step.is_done());
        let mut items = tp.items();
        items.sort();
        assert_eq!(items, vec![(1, 10), (2, 20)]);
    }
}
