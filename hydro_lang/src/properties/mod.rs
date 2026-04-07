//! Types for reasoning about algebraic properties for Rust closures.

use std::marker::PhantomData;

use stageleft::properties::Property;

use crate::live_collections::boundedness::Boundedness;
use crate::live_collections::keyed_singleton::KeyedSingletonBound;
use crate::live_collections::singleton::SingletonBound;
use crate::live_collections::stream::{ExactlyOnce, Ordering, Retries, TotalOrder};

/// A trait for proof mechanisms that can validate commutativity.
#[sealed::sealed]
pub trait CommutativeProof {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);
}

/// A trait for proof mechanisms that can validate idempotence.
#[sealed::sealed]
pub trait IdempotentProof {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);
}

/// A trait for proof mechanisms that can validate monotonicity.
#[sealed::sealed]
pub trait MonotoneProof {
    /// Registers the expression with the proof mechanism.
    ///
    /// This should not perform any blocking analysis; it is only intended to record the expression for later processing.
    fn register_proof(&self, expr: &syn::Expr);
}

/// A hand-written human proof of the correctness property.
///
/// To create a manual proof, use the [`manual_proof!`] macro, which takes in a doc comment
/// explaining why the property holds.
pub struct ManualProof();
#[sealed::sealed]
impl CommutativeProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}
#[sealed::sealed]
impl IdempotentProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}
#[sealed::sealed]
impl MonotoneProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}

#[doc(inline)]
pub use crate::__manual_proof__ as manual_proof;

#[macro_export]
/// Fulfills a proof parameter by declaring a human-written justification for why
/// the algebraic property (e.g. commutativity, idempotence) holds.
///
/// The argument must be a doc comment explaining why the property is satisfied.
///
/// # Examples
/// ```rust,ignore
/// use hydro_lang::prelude::*;
///
/// stream.fold(
///     q!(|| 0),
///     q!(
///         |acc, x| *acc += x,
///         commutative = manual_proof!(/** integer addition is commutative */)
///     )
/// )
/// ```
macro_rules! __manual_proof__ {
    ($(#[doc = $doc:expr])+) => {
        $crate::properties::ManualProof()
    };
}

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
pub struct AggFuncAlgebra<Commutative = NotProved, Idempotent = NotProved, Monotone = NotProved>(
    Option<Box<dyn CommutativeProof>>,
    Option<Box<dyn IdempotentProof>>,
    Option<Box<dyn MonotoneProof>>,
    PhantomData<(Commutative, Idempotent, Monotone)>,
);

impl<C, I, M> AggFuncAlgebra<C, I, M> {
    /// Marks the function as being commutative, with the given proof mechanism.
    pub fn commutative(
        self,
        proof: impl CommutativeProof + 'static,
    ) -> AggFuncAlgebra<Proved, I, M> {
        AggFuncAlgebra(Some(Box::new(proof)), self.1, self.2, PhantomData)
    }

    /// Marks the function as being idempotent, with the given proof mechanism.
    pub fn idempotent(self, proof: impl IdempotentProof + 'static) -> AggFuncAlgebra<C, Proved, M> {
        AggFuncAlgebra(self.0, Some(Box::new(proof)), self.2, PhantomData)
    }

    /// Marks the function as being monotone, with the given proof mechanism.
    pub fn monotone(self, proof: impl MonotoneProof + 'static) -> AggFuncAlgebra<C, I, Proved> {
        AggFuncAlgebra(self.0, self.1, Some(Box::new(proof)), PhantomData)
    }

    /// Registers the expression with the underlying proof mechanisms.
    pub(crate) fn register_proof(self, expr: &syn::Expr) {
        if let Some(comm_proof) = self.0 {
            comm_proof.register_proof(expr);
        }

        if let Some(idem_proof) = self.1 {
            idem_proof.register_proof(expr);
        }

        if let Some(monotone_proof) = self.2 {
            monotone_proof.register_proof(expr);
        }
    }
}

impl<C, I, M> Property for AggFuncAlgebra<C, I, M> {
    type Root = AggFuncAlgebra;

    fn make_root(_target: &mut Option<Self>) -> Self::Root {
        AggFuncAlgebra(None, None, None, PhantomData)
    }
}

/// Marker trait identifying that the commutativity property is valid for the given stream ordering.
#[diagnostic::on_unimplemented(
    message = "Because the input stream has ordering `{O}`, the closure must demonstrate commutativity with a `commutative = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing a non-deterministic (shuffled) order of elements, use `.assume_ordering`. This introduces non-determinism so avoid unless necessary."
)]
#[sealed::sealed]
pub trait ValidCommutativityFor<O: Ordering> {}
#[sealed::sealed]
impl ValidCommutativityFor<TotalOrder> for NotProved {}
#[sealed::sealed]
impl<O: Ordering> ValidCommutativityFor<O> for Proved {}

/// Marker trait identifying that the idempotence property is valid for the given stream ordering.
#[diagnostic::on_unimplemented(
    message = "Because the input stream has retries `{R}`, the closure must demonstrate idempotence with an `idempotent = ...` annotation.",
    label = "required for this call",
    note = "To intentionally process the stream by observing non-deterministic (randomly duplicated) retries, use `.assume_retries`. This introduces non-determinism so avoid unless necessary."
)]
#[sealed::sealed]
pub trait ValidIdempotenceFor<R: Retries> {}
#[sealed::sealed]
impl ValidIdempotenceFor<ExactlyOnce> for NotProved {}
#[sealed::sealed]
impl<R: Retries> ValidIdempotenceFor<R> for Proved {}

/// Marker trait identifying the boundedness of a singleton given a monotonicity property of
/// an aggregation on a stream.
#[sealed::sealed]
pub trait ApplyMonotoneStream<P, B2: SingletonBound> {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneStream<NotProved, B> for B {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneStream<Proved, B::StreamToMonotone> for B {}

/// Marker trait identifying the boundedness of a singleton given a monotonicity property of
/// an aggregation on a keyed stream.
#[sealed::sealed]
pub trait ApplyMonotoneKeyedStream<P, B2: KeyedSingletonBound> {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneKeyedStream<NotProved, B> for B {}

#[sealed::sealed]
impl<B: Boundedness> ApplyMonotoneKeyedStream<Proved, B::KeyedStreamToMonotone> for B {}
