#![deny(missing_docs)]
#![feature(impl_trait_in_assoc_type)]
#![doc = include_str!("../README.md")]

use std::cmp::Ordering::{self, *};

use sealed::sealed;

mod bottom;
pub mod collections;
pub mod test;
pub use bottom::Bottom;
mod dom_pair;
pub use dom_pair::DomPair;
mod fake;
pub use fake::Fake;
pub mod map_union;
mod ord;
pub use ord::{Max, Min};
mod pair;
pub use pair::Pair;
pub mod cc_traits;
pub mod set_union;

/// Trait for lattice merge (AKA "join" or "least upper bound").
pub trait Merge<Other> {
    /// Merge `other` into the `self` lattice.
    ///
    /// This operation must be associative, commutative, and idempotent.
    ///
    /// Returns `true` if `self` changed, `false` otherwise.
    /// Returning `true` implies that the new value for `self` is later in the lattice order than
    /// the old value. Returning `false` means that `self` was unchanged and therefore `other` came
    /// before `self` (or the two are equal) in the lattice order.
    fn merge(&mut self, other: Other) -> bool;

    /// Merge `this` and `delta` together, returning the new value.
    fn merge_owned(mut this: Self, delta: Other) -> Self
    where
        Self: Sized,
    {
        Self::merge(&mut this, delta);
        this
    }
}

/// Trait for lattice partial order comparison
/// PartialOrd is implemented for many things, this trait can be used to require the type be a lattice.
pub trait LatticeOrd<Rhs = Self>: PartialOrd<Rhs> {}

/// Naive lattice compare, based on the [`Merge::merge`] function.
#[sealed]
pub trait NaiveLatticeOrd<Rhs = Self>
where
    Self: Clone + Merge<Rhs> + Sized,
    Rhs: Clone + Merge<Self>,
{
    /// Naive compare based on the [`Merge::merge`] method. This method can be very inefficient;
    /// use [`PartialOrd::partial_cmp`] instead.
    ///
    /// This method should not be overridden.
    fn naive_cmp(&self, other: &Rhs) -> Option<Ordering> {
        let mut self_a = self.clone();
        let other_a = other.clone();
        let self_b = self.clone();
        let mut other_b = other.clone();
        match (self_a.merge(other_a), other_b.merge(self_b)) {
            (true, true) => None,
            (true, false) => Some(Less),
            (false, true) => Some(Greater),
            (false, false) => Some(Equal),
        }
    }
}
#[sealed]
impl<This, Other> NaiveLatticeOrd<Other> for This
where
    Self: Clone + Merge<Other>,
    Other: Clone + Merge<Self>,
{
}

/// Same as `From` but for lattices.
///
/// This should only be implemented between different representations of the same lattice type.
/// This should recursively convert nested lattice types, but not non-lattice ("scalar") types.
pub trait ConvertFrom<Other> {
    /// Convert from the `Other` lattice into `Self`.
    fn from(other: Other) -> Self;
}
