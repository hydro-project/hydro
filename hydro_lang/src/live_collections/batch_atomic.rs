use super::boundedness::{Bounded, Unbounded};
use crate::live_collections::keyed_singleton::KeyedSingletonBound;
use crate::live_collections::singleton::SingletonBound;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::tick::Tick;
use crate::location::{Atomic, Location};
use crate::nondet::nondet;

/// Helper trait for live collections which can be batched back into a tick from a matching
/// atomic region. Used in [`super::Stream::across_ticks`]
pub trait BatchAtomic<'a> {
    /// The type of the stream when returned to the tick.
    type Batched;

    /// Batches / Snapshots the atomic live collection back into its corresponding tick.
    fn batched_atomic(self) -> Self::Batched;
}

impl<'a, L: Location<'a>, T, O: Ordering, R: Retries> BatchAtomic<'a>
    for super::Stream<T, Atomic<L>, Unbounded, O, R>
{
    type Batched = super::Stream<T, Tick<L::DropConsistency>, Bounded, O, R>;

    fn batched_atomic(self) -> Self::Batched {
        let tick = self.location.tick.clone();
        self.batch_atomic(&tick, nondet!(/** internal */))
    }
}

impl<'a, L: Location<'a>, T, B: SingletonBound> BatchAtomic<'a>
    for super::Singleton<T, Atomic<L>, B>
{
    type Batched = super::Singleton<T, Tick<L::DropConsistency>, Bounded>;

    fn batched_atomic(self) -> Self::Batched {
        let tick = self.location.tick.clone();
        self.snapshot_atomic(&tick, nondet!(/** internal */))
    }
}

impl<'a, L: Location<'a>, T> BatchAtomic<'a> for super::Optional<T, Atomic<L>, Unbounded> {
    type Batched = super::Optional<T, Tick<L::DropConsistency>, Bounded>;

    fn batched_atomic(self) -> Self::Batched {
        let tick = self.location.tick.clone();
        self.snapshot_atomic(&tick, nondet!(/** internal */))
    }
}

impl<'a, L: Location<'a>, K, V, B: KeyedSingletonBound<ValueBound = Unbounded>> BatchAtomic<'a>
    for super::KeyedSingleton<K, V, Atomic<L>, B>
{
    type Batched = super::KeyedSingleton<K, V, Tick<L::DropConsistency>, Bounded>;

    fn batched_atomic(self) -> Self::Batched {
        let tick = self.location.tick.clone();
        self.snapshot_atomic(&tick, nondet!(/** internal */))
    }
}
