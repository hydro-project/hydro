use stageleft::q;

use crate::cycle::{CycleCollection, CycleComplete, ForwardRefMarker};
use crate::location::tick::NoAtomic;
use crate::location::{LocationId, NoTick};
use crate::stream::ExactlyOnce;
use crate::{Bounded, Location, NoOrder, Stream, Tick, Unbounded};

pub struct KeyedOptional<K, V, Loc, Bound> {
    pub(crate) underlying: Stream<(K, V), Loc, Bound, NoOrder, ExactlyOnce>,
}

impl<'a, K: Clone, V: Clone, Loc: Location<'a>, Bound> Clone for KeyedOptional<K, V, Loc, Bound> {
    fn clone(&self) -> Self {
        KeyedOptional {
            underlying: self.underlying.clone(),
        }
    }
}

impl<'a, K, V, L, B> CycleCollection<'a, ForwardRefMarker> for KeyedOptional<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    type Location = L;

    fn create_source(ident: syn::Ident, location: L) -> Self {
        KeyedOptional {
            underlying: Stream::create_source(ident, location),
        }
    }
}

impl<'a, K, V, L, B> CycleComplete<'a, ForwardRefMarker> for KeyedOptional<K, V, L, B>
where
    L: Location<'a> + NoTick,
{
    fn complete(self, ident: syn::Ident, expected_location: LocationId) {
        self.underlying.complete(ident, expected_location);
    }
}

impl<'a, K, V, L: Location<'a>> KeyedOptional<K, V, Tick<L>, Bounded> {
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

impl<'a, K, V, L, B> KeyedOptional<K, V, L, B>
where
    L: Location<'a> + NoTick + NoAtomic,
{
    /// Given a tick, returns a keyed optional with a entries consisting of keys with
    /// snapshots of the value optional.
    ///
    /// # Safety
    /// Because this picks a snapshot of each singleton whose value is continuously changing,
    /// the output singleton has a non-deterministic value since each snapshot can be at an
    /// arbitrary point in time.
    pub unsafe fn tick_batch(self, tick: &Tick<L>) -> KeyedOptional<K, V, Tick<L>, Bounded> {
        KeyedOptional {
            underlying: unsafe { self.underlying.tick_batch(tick) },
        }
    }
}

impl<'a, K, V, L> KeyedOptional<K, V, Tick<L>, Bounded>
where
    L: Location<'a>,
{
    pub fn all_ticks(self) -> KeyedOptional<K, V, L, Unbounded> {
        KeyedOptional {
            underlying: self.underlying.all_ticks(),
        }
    }
}
