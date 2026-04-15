//! Type declarations for boundedness markers, which indicate whether a live collection is finite
//! and immutable ([`Bounded`]) or asynchronously arriving over time ([`Unbounded`]).

use sealed::sealed;

use super::keyed_singleton::KeyedSingletonBound;
use crate::compile::ir::BoundKind;
use crate::live_collections::singleton::SingletonBound;

/// A marker trait indicating whether a stream’s length is bounded (finite) or unbounded (potentially infinite).
///
/// Implementors of this trait use it to signal the boundedness property of a stream.
#[sealed]
pub trait Boundedness:
    SingletonBound<UnderlyingBound = Self> + KeyedSingletonBound<UnderlyingBound = Self>
{
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

#[sealed]
#[diagnostic::on_unimplemented(
    message = "The input collection must be bounded (`Bounded`), but has bound `{Self}`. Strengthen the boundedness upstream or consider a different API.",
    label = "required here",
    note = "To intentionally process a non-deterministic snapshot or batch, you may want to use a `sliced!` region. This introduces non-determinism so avoid unless necessary."
)]
/// Marker trait that is implemented for the [`Bounded`] boundedness guarantee.
pub trait IsBounded: Boundedness {}

#[sealed]
#[diagnostic::do_not_recommend]
impl IsBounded for Bounded {}
