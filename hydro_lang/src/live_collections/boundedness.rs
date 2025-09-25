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
    /// Returns `true` if the bound is [`Bounded`], `false` if it is [`Unbounded`].
    fn is_bounded() -> bool;

    /// Returns the [`BoundKind`] corresponding to this type.
    fn bound_kind() -> BoundKind {
        if Self::is_bounded() {
            BoundKind::Bounded
        } else {
            BoundKind::Unbounded
        }
    }
}

/// Marks the stream as being unbounded, which means that it is not
/// guaranteed to be complete in finite time.
pub enum Unbounded {}

#[sealed]
impl Boundedness for Unbounded {
    fn is_bounded() -> bool {
        false
    }
}

/// Marks the stream as being bounded, which means that it is guaranteed
/// to be complete in finite time.
pub enum Bounded {}

#[sealed]
impl Boundedness for Bounded {
    fn is_bounded() -> bool {
        true
    }
}

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
