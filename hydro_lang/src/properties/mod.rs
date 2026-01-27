//! Types for reasoning about algebraic properties for Rust closures.

use std::marker::PhantomData;

use stageleft::properties::Property;

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

/// A hand-written human proof of the correctness property.
pub struct ManualProof();
#[sealed::sealed]
impl CommutativeProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
}
#[sealed::sealed]
impl IdempotentProof for ManualProof {
    fn register_proof(&self, _expr: &syn::Expr) {}
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
pub struct AggFuncAlgebra<Commutative = NotProved, Idempotent = NotProved>(
    Option<Box<dyn CommutativeProof>>,
    Option<Box<dyn IdempotentProof>>,
    PhantomData<(Commutative, Idempotent)>,
);

impl<C, I> AggFuncAlgebra<C, I> {
    /// Marks the function as being commutative, with the given proof mechanism.
    pub fn commutative(self, proof: impl CommutativeProof + 'static) -> AggFuncAlgebra<Proved, I> {
        AggFuncAlgebra(Some(Box::new(proof)), self.1, PhantomData)
    }

    /// Marks the function as being idempotent, with the given proof mechanism.
    pub fn idempotent(self, proof: impl IdempotentProof + 'static) -> AggFuncAlgebra<C, Proved> {
        AggFuncAlgebra(self.0, Some(Box::new(proof)), PhantomData)
    }

    /// Registers the expression with the underlying proof mechanisms.
    pub(crate) fn register_proof(self, expr: &syn::Expr) {
        if let Some(comm_proof) = self.0 {
            comm_proof.register_proof(expr);
        }

        if let Some(idem_proof) = self.1 {
            idem_proof.register_proof(expr);
        }
    }
}

impl<C, I> Property for AggFuncAlgebra<C, I> {
    type Root = AggFuncAlgebra;

    fn make_root(_target: &mut Option<Self>) -> Self::Root {
        AggFuncAlgebra(None, None, PhantomData)
    }
}

/// Marker trait identifying that the commutativity property is valid for the given stream ordering.
#[diagnostic::on_unimplemented(
    message = "Because the input stream has ordering `{O}`, the closure must demonstrate commutativity with a `commutative = ...` annotation.",
    label = "required for this call"
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
    label = "required for this call"
)]
#[sealed::sealed]
pub trait ValidIdempotenceFor<R: Retries> {}
#[sealed::sealed]
impl ValidIdempotenceFor<ExactlyOnce> for NotProved {}
#[sealed::sealed]
impl<R: Retries> ValidIdempotenceFor<R> for Proved {}
