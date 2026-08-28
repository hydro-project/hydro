//! Type declarations for boundedness markers, which indicate whether a live collection is finite
//! and immutable ([`Bounded`]) or asynchronously arriving over time ([`Unbounded`]).

use sealed::sealed;

use super::keyed_singleton::KeyedSingletonBound;
use super::optional::{InitNone, OptionalBound};
use crate::compile::ir::BoundKind;
use crate::live_collections::singleton::SingletonBound;

/// A marker trait indicating whether a stream's length is bounded (finite) or unbounded (potentially infinite).
///
/// Implementors of this trait use it to signal the boundedness property of a stream.
#[sealed]
pub trait Boundedness:
    SingletonBound<UnderlyingBound = Self>
    + KeyedSingletonBound<UnderlyingBound = Self>
    + OptionalBound<UnderlyingBound = Self>
{
    /// `true` if the bound is [`Bounded`], `false` if it is [`Unbounded`].
    const BOUNDED: bool;

    /// The [`BoundKind`] corresponding to this type.
    const BOUND_KIND: BoundKind = if Self::BOUNDED {
        BoundKind::Bounded
    } else {
        BoundKind::Unbounded
    };

    /// The [`OptionalBound`] of an optional produced by aggregating a stream with this
    /// boundedness (e.g. `reduce`/`max`/`min`/`first`/`last`).
    ///
    /// A [`Bounded`] stream yields a [`Bounded`] optional. An [`Unbounded`] stream yields an
    /// [`InitNone`] optional: such an aggregation is materialized at a top-level location where
    /// its state persists across ticks, so once the first element arrives the result becomes
    /// non-null and stays non-null (its value may still change).
    type AggregatedOptional: OptionalBound<UnderlyingBound = Self>;

    /// Determines the output ordering of a join based on this (right/build) side's boundedness.
    ///
    /// When this side is [`Bounded`], the join accumulates this side first and then
    /// streams the left side through, preserving the left side's ordering `InO`.
    /// When this side is [`Unbounded`], a symmetric hash join is used and ordering is lost.
    type PreserveOrderIfBounded<InO: crate::live_collections::stream::Ordering>: crate::live_collections::stream::Ordering;
}

/// Marks the stream as being unbounded, which means that it is not
/// guaranteed to be complete in finite time.
pub enum Unbounded {}

#[sealed]
impl Boundedness for Unbounded {
    const BOUNDED: bool = false;
    type AggregatedOptional = InitNone;
    type PreserveOrderIfBounded<InO: crate::live_collections::stream::Ordering> =
        crate::live_collections::stream::NoOrder;
}

/// Marks the stream as being bounded, which means that it is guaranteed
/// to be complete in finite time.
pub enum Bounded {}

#[sealed]
impl Boundedness for Bounded {
    const BOUNDED: bool = true;
    type AggregatedOptional = Bounded;
    type PreserveOrderIfBounded<InO: crate::live_collections::stream::Ordering> = InO;
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
