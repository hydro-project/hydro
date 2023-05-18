#![deny(missing_docs)]
#![feature(impl_trait_in_assoc_type)]
//! Module containing lattice traits and implementations.
//!
//! Convention: Generic parameters that are full words (e.g. `Other`, `Key`, `Val`) are lattice
//! types.
//! Conversely, Generic parameters that are single letters or acronyms (e.g. `K`, `T`) are scalar
//! non-`Lattice` types.

use std::cmp::Ordering::{self, *};

use sealed::sealed;

pub mod bottom;
pub mod collections;
pub mod dom_pair;
pub mod fake;
pub mod map_union;
pub mod ord;
pub mod pair;
pub mod set_union;
pub mod test;

/// Re-export of the [`cc_traits`](::cc_traits) crate with [`SimpleKeyedRef`](cc_traits::SimpleKeyedRef) added.
pub mod cc_traits {
    pub use ::cc_traits::*;

    /// <https://github.com/timothee-haudebourg/cc-traits/pull/8>
    ///
    /// Keyed collection where each key reference can be converted into a standard
    /// "simple" rust reference.
    ///
    /// This trait is particularly useful to avoid having to include where bounds
    /// of the form `for<'r> T::KeyRef<'r>: Into<&'r T::Key>`, which can
    /// currently lead the compiler to try to prove `T: 'static`
    /// (see <https://github.com/rust-lang/rust/pull/96709#issuecomment-1182403490>)
    /// for more details.
    pub trait SimpleKeyedRef: KeyedRef {
        /// Convert the borrow into a simple `&Key` ref.
        fn into_ref<'r>(r: Self::KeyRef<'r>) -> &'r Self::Key
        where
            Self: 'r;
    }
    impl<T> SimpleKeyedRef for T
    where
        T: KeyedRef,
        for<'a> Self::KeyRef<'a>: Into<&'a Self::Key>,
    {
        fn into_ref<'r>(r: Self::KeyRef<'r>) -> &'r Self::Key
        where
            Self: 'r,
        {
            r.into()
        }
    }
}

/// Trait for lattice merge (least upper bound).
pub trait Merge<Other> {
    /// Merge `other` into the `self` lattice.
    ///
    /// Returns whether `self` changed at all.
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
pub trait NaiveOrd<Rhs = Self>
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
impl<This, Other> NaiveOrd<Other> for This
where
    Self: Clone + Merge<Other>,
    Other: Clone + Merge<Self>,
{
}

/// Same as `From` but for lattices.
///
/// Do not convert non-lattice (AKA scalar) types if you implement this trait.
pub trait ConvertFrom<Other> {
    /// Convert from the `Other` lattice into `Self`.
    fn from(other: Other) -> Self;
}
