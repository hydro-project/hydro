//! Types for reasoning about algebraic properties for Rust closures.

use std::marker::PhantomData;

use stageleft::properties::Property;

use crate::live_collections::stream::{ExactlyOnce, Ordering, Retries, TotalOrder};

/// A trait for proof mechanisms that can validate commutativity.
pub trait CommutativeProof {}

/// A trait for proof mechanisms that can validate idempotence.
pub trait IdempotentProof {}

/// A hand-written human proof of the correctness property.
pub struct ManualProof();
impl CommutativeProof for ManualProof {}
impl IdempotentProof for ManualProof {}

/// Marks that the property is not proved.
pub enum NotProved {}

/// Marks that the property is proven.
pub enum Proved {}

/// Algebraic properties for an aggregation function of type (T, &mut A) -> ().
///
/// Commutativity:
/// ```rust,ignore
/// let mut state = ???;
/// f(a, &mut state); f(b, &mut state) // produces same final state as
/// f(b, &mut state); f(a, &mut state)
/// ```
///
/// Idempotence:
/// ```rust,ignore
/// let mut state = ???;
/// f(a, &mut state);
/// let state1 = *state;
/// f(a, &mut state);
/// // state1 must be equal to state
/// ```
pub struct AggFuncAlgebra<Commutative = NotProved, Idempotent = NotProved>(
    PhantomData<(Commutative, Idempotent)>,
);
impl<C, I> AggFuncAlgebra<C, I> {
    /// Marks the function as being commutative, with the given proof mechanism.
    pub fn commutative(self, _proof: impl CommutativeProof) -> AggFuncAlgebra<Proved, I> {
        AggFuncAlgebra(PhantomData)
    }

    /// Marks the function as being idempotent, with the given proof mechanism.
    pub fn idempotent(self, _proof: impl IdempotentProof) -> AggFuncAlgebra<C, Proved> {
        AggFuncAlgebra(PhantomData)
    }
}

impl<C, I> Property for AggFuncAlgebra<C, I> {
    type Root = AggFuncAlgebra;

    fn make_root(_target: &mut Option<Self>) -> Self::Root {
        AggFuncAlgebra(PhantomData)
    }
}

/// Marker trait identifying that the commutativity property is valid for the given stream ordering.
pub trait ValidCommutativityFor<O: Ordering> {}
impl ValidCommutativityFor<TotalOrder> for NotProved {}
impl<O: Ordering> ValidCommutativityFor<O> for Proved {}

/// Marker trait identifying that the idempotence property is valid for the given stream ordering.
pub trait ValidIdempotenceFor<R: Retries> {}
impl ValidIdempotenceFor<ExactlyOnce> for NotProved {}
impl<R: Retries> ValidIdempotenceFor<R> for Proved {}
