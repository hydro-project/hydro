use sealed::sealed;

use crate::live_collections::keyed_singleton::{BoundedValue, KeyedSingletonBound};

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
    /// The boundedness of the keyed singleton if the values for each key will asynchronously change.
    type WhenValueUnbounded: KeyedSingletonBound<UnderlyingBound = Self>;
    /// The boundedness of the keyed singleton if the value for each key is immutable.
    type WhenValueBounded: KeyedSingletonBound<UnderlyingBound = Self>;
}

#[sealed::sealed]
impl KeyedBoundFoldLike for Unbounded {
    type WhenValueUnbounded = Unbounded;
    type WhenValueBounded = BoundedValue;
}

#[sealed::sealed]
impl KeyedBoundFoldLike for Bounded {
    type WhenValueUnbounded = Bounded;
    type WhenValueBounded = Bounded;
}
