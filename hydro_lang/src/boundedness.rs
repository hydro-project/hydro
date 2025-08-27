use sealed::sealed;

use crate::keyed_singleton::{KeyedSingletonBound, ValueBounded};

/// A marker trait indicating whether a stream’s length is bounded (finite) or unbounded (potentially infinite).
///
/// Implementors of this trait use it to signal the boundedness property of a stream.
#[sealed]
pub trait Boundedness: KeyedBoundFoldLike {}

/// Marks the stream as being unbounded, which means that it is not
/// guaranteed to be complete in finite time.
pub enum Unbounded {}

#[sealed]
impl Boundedness for Unbounded {}

/// Marks the stream as being bounded, which means that it is guaranteed
/// to be complete in finite time.
pub enum Bounded {}

#[sealed]
impl Boundedness for Bounded {}

/// Helper trait that determines the boundedness for the result of keyed aggregations.
#[sealed::sealed]
pub trait KeyedBoundFoldLike {
    type KeyedBound: KeyedSingletonBound;
}

#[sealed::sealed]
impl KeyedBoundFoldLike for Unbounded {
    type KeyedBound = Unbounded;
}

#[sealed::sealed]
impl KeyedBoundFoldLike for Bounded {
    type KeyedBound = ValueBounded<Bounded>;
}
