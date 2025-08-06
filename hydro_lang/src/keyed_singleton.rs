use stageleft::{IntoQuotedMut, QuotedWithContext, q};

use crate::cycle::{CycleCollection, CycleComplete, ForwardRefMarker};
use crate::keyed_optional::KeyedOptional;
use crate::location::tick::NoAtomic;
use crate::location::{LocationId, NoTick};
use crate::manual_expr::ManualExpr;
use crate::stream::ExactlyOnce;
use crate::{Atomic, Bounded, Location, NoOrder, Stream, Tick, Unbounded};

pub struct KeyedSingleton<K, V, Loc, Bound> {
    pub(crate) underlying: Stream<(K, V), Loc, Bound, NoOrder, ExactlyOnce>,
}

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound> Clone for KeyedSingleton<K, V, Loc, Bound> {
    fn clone(&self) -> Self {
        KeyedSingleton {
            underlying: self.underlying.clone(),
        }
    }
}

impl<'a, K, V, L, B> CycleCollection<'a, ForwardRefMarker> for KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        KeyedSingleton {
            underlying: Stream::create_source(ident, location),
        }
    }
}

impl<'a, K, V, L, B> CycleComplete<'a, ForwardRefMarker> for KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        self.underlying.complete(ident, expected_location);
    }
}

impl<'a, K, V, L: Location<'a>> KeyedSingleton<K, V, Tick<L>, Bounded> {
    pub fn entries(self) -> Stream<(K, V), Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.underlying
    }

    pub fn values(self) -> Stream<V, Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.underlying.map(q!(|(_, v)| v))
    }

    pub fn keys(self) -> Stream<K, Tick<L>, Bounded, NoOrder, ExactlyOnce> {
        self.underlying.map(q!(|(k, _)| k))
    }
}

impl<'a, K, V, L: Location<'a>, B> KeyedSingleton<K, V, L, B> {
    pub fn map<U, F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedSingleton<K, U, L, B>
    where
        F: Fn(V) -> U + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedSingleton {
            underlying: self.underlying.map(q!({
                let orig = f;
                move |(k, v)| (k, orig(v))
            })),
        }
    }

    pub fn filter<F>(self, f: impl IntoQuotedMut<'a, F, L> + Copy) -> KeyedOptional<K, V, L, B>
    where
        F: Fn(&V) -> bool + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_borrow_ctx(ctx));
        KeyedOptional {
            underlying: self.underlying.filter(q!({
                let orig = f;
                move |(_k, v)| orig(v)
            })),
        }
    }

    pub fn filter_map<F, U>(
        self,
        f: impl IntoQuotedMut<'a, F, L> + Copy,
    ) -> KeyedOptional<K, U, L, B>
    where
        F: Fn(V) -> Option<U> + 'a,
    {
        let f: ManualExpr<F, _> = ManualExpr::new(move |ctx: &L| f.splice_fn1_ctx(ctx));
        KeyedOptional {
            underlying: self.underlying.filter_map(q!({
                let orig = f;
                move |(k, v)| orig(v).map(|v| (k, v))
            })),
        }
    }
}

impl<'a, K, V, L, B> KeyedSingleton<K, V, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    pub fn atomic(self, tick: &Tick<L>) -> KeyedSingleton<K, V, Atomic<L>, B> {
        KeyedSingleton {
            underlying: self.underlying.atomic(tick),
        }
    }

    /// Given a tick, returns a keyed singleton with a entries consisting of keys with
    /// snapshots of the value singleton.
    ///
    /// # Safety
    /// Because this picks a snapshot of each singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since each snapshot can be at an
    /// arbitrary point in time.
    pub unsafe fn snapshot(self, tick: &Tick<L>) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        unsafe { self.atomic(tick).snapshot() }
    }
}

impl<'a, K, V, L, B> KeyedSingleton<K, V, Atomic<L>, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Returns a keyed singleton with a entries consisting of keys with snapshots of the value
    /// singleton being atomically processed.
    ///
    /// # Safety
    /// Because this picks a snapshot of each singleton whose value is continuously changing,
    /// each output singleton has a non-deterministic value since each snapshot can be at an
    /// arbitrary point in time.
    pub unsafe fn snapshot(self) -> KeyedSingleton<K, V, Tick<L>, Bounded> {
        KeyedSingleton {
            underlying: Stream::new(
                self.underlying.location.tick,
                // no need to unpersist due to top-level replay
                self.underlying.ir_node.into_inner(),
            ),
        }
    }
}

impl<'a, K, V, L> KeyedSingleton<K, V, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    pub fn all_ticks(self) -> KeyedSingleton<K, V, L, Unbounded> {
        KeyedSingleton {
            underlying: self.underlying.all_ticks(),
        }
    }
}
