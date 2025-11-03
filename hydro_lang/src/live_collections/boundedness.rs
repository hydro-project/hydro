//! Type declarations for boundedness markers, which indicate whether a live collection is finite
//! and immutable ([`Bounded`]) or asynchronously arriving over time ([`Unbounded`]).

use sealed::sealed;

use super::keyed_singleton::{BoundedValue, KeyedSingletonBound};
use crate::compile::ir::BoundKind;

/// A marker trait indicating whether a stream’s length is bounded (finite) or unbounded (potentially infinite).
///
/// Implementors of this trait use it to signal the boundedness property of a stream.
#[sealed]
pub trait Boundedness: KeyedBoundFoldLike {
    /// `true` if the bound is [`Bounded`], `false` if it is [`Unbounded`].
    const BOUNDED: bool;

    /// The [`BoundKind`] corresponding to this type.
    const BOUND_KIND: BoundKind = if Self::BOUNDED {
        BoundKind::Bounded
    } else {
        BoundKind::Unbounded
    };
}

/// Marks the stream as being unbounded, which means that it is not
/// guaranteed to be complete in finite time.
pub enum Unbounded {}

#[sealed]
impl Boundedness for Unbounded {
    const BOUNDED: bool = false;
}

/// Marks the stream as being bounded, which means that it is guaranteed
/// to be complete in finite time.
pub enum Bounded {}

#[sealed]
impl Boundedness for Bounded {
    const BOUNDED: bool = true;
}

/// Helper trait that determines the boundedness for the result of keyed aggregations.
#[sealed]
pub trait KeyedBoundFoldLike {
    /// The boundedness of the keyed singleton if the values for each key will asynchronously change.
    type WhenValueUnbounded: KeyedSingletonBound<UnderlyingBound = Self>;
    /// The boundedness of the keyed singleton if the value for each key is immutable.
    type WhenValueBounded: KeyedSingletonBound<UnderlyingBound = Self>;
}

#[sealed]
impl KeyedBoundFoldLike for Unbounded {
    type WhenValueUnbounded = Unbounded;
    type WhenValueBounded = BoundedValue;
}

#[sealed]
impl KeyedBoundFoldLike for Bounded {
    type WhenValueUnbounded = Bounded;
    type WhenValueBounded = Bounded;
}
