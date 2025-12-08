use super::boundedness::{Bounded, Unbounded};
use crate::live_collections::keyed_singleton::KeyedSingletonBound;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::tick::Tick;
use crate::location::{Atomic, Location, NoTick};
use crate::nondet::nondet;

/// Helper trait for live collections which can be batched back into a tick from a matching
/// atomic region. Used in [`super::Stream::across_ticks`]
pub trait BatchAtomic {
    /// The type of the stream when returned to the tick.
    type Batched;

    /// Batches / Snapshots the atomic live collection back into its corresponding tick.
    fn batched_atomic(self) -> Self::Batched;
}

impl<'a, L: Location<'a> + NoTick, T, O: Ordering, R: Retries> BatchAtomic
    for super::Stream<T, Atomic<L>, Unbounded, O, R>
{
    type Batched = super::Stream<T, Tick<L>, Bounded, O, R>;

    fn batched_atomic(self) -> Self::Batched {
        self.batch_atomic(nondet!(/** internal */))
    }
}

impl<'a, L: Location<'a> + NoTick, T> BatchAtomic for super::Singleton<T, Atomic<L>, Unbounded> {
    type Batched = super::Singleton<T, Tick<L>, Bounded>;

    fn batched_atomic(self) -> Self::Batched {
        self.snapshot_atomic(nondet!(/** internal */))
    }
}

impl<'a, L: Location<'a> + NoTick, T> BatchAtomic for super::Optional<T, Atomic<L>, Unbounded> {
    type Batched = super::Optional<T, Tick<L>, Bounded>;

    fn batched_atomic(self) -> Self::Batched {
        self.snapshot_atomic(nondet!(/** internal */))
    }
}

impl<'a, L: Location<'a> + NoTick, K, V, B: KeyedSingletonBound<ValueBound = Unbounded>> BatchAtomic
    for super::KeyedSingleton<K, V, Atomic<L>, B>
{
    type Batched = super::KeyedSingleton<K, V, Tick<L>, Bounded>;

    fn batched_atomic(self) -> Self::Batched {
        self.snapshot_atomic(nondet!(/** internal */))
    }
}
