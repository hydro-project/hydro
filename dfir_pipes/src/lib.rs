//! Pull and push-based stream combinators for dataflow pipelines.
//!
//! This crate provides a [`pull::Pull`] trait and a [`push::Push`] trait, along with collections
//! of composable operators for building pull-based and push-based data pipelines.
//! Operators are chained via trait methods on [`pull::Pull`] (same as iterator adapters) or
//! module functions in [`push`].
#![no_std]
#![cfg_attr(nightly, feature(extend_one))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs, clippy::missing_const_for_fn)]

#[cfg(any(test, feature = "alloc"))]
extern crate alloc;
#[cfg(any(test, feature = "std"))]
extern crate std;

/// Type-level `false` for [`Toggle`].
///
/// Indicates that a capability is absent (e.g., the pull cannot pend or cannot end).
///
/// A type alias for `core::convert::Infallible`, representing a type that can never be constructed.
///
/// Used in `Step` variants that are statically impossible (e.g., `Pending` when `CanPend = No`).
pub use core::convert::Infallible as No;

pub use futures_core::stream::{FusedStream, Stream};
pub use futures_sink::Sink;
pub use itertools::{self, Either, EitherOrBoth};
use sealed::sealed;

/// Pull-based stream combinators.
pub mod pull;

/// Push-based stream combinators.
pub mod push;

/// A sealed trait for type-level booleans used to track pull capabilities.
///
/// `Toggle` is used to statically encode whether a pull can pend (`CanPend`) or end (`CanEnd`).
/// This enables compile-time guarantees about pull behavior and allows the type system to
/// optimize away impossible code paths.
#[sealed]
pub trait Toggle: Sized {
    /// Attempts to create this type, returning `Err(())` if `Self` is `No`.
    fn try_create() -> Option<Self>;

    /// Attempts to create this type, panicking if `Self` is `No`.
    fn create() -> Self {
        Self::try_create().unwrap()
    }

    /// The result of OR-ing two toggles. `Yes.or(T) = Yes`, `No.or(T) = T`.
    type Or<T: Toggle>: Toggle;
    /// The result of AND-ing two toggles. `Yes.and(T) = T`, `No.and(T) = No`.
    type And<T: Toggle>: Toggle;
}

/// Type-level `true` for [`Toggle`].
///
/// Indicates that a capability is present (e.g., the pull can pend or can end).
#[derive(Default)]
pub struct Yes;

#[sealed]
impl Toggle for Yes {
    fn try_create() -> Option<Self> {
        Some(Yes)
    }

    type Or<T: Toggle> = Yes;
    type And<T: Toggle> = T;
}

#[sealed]
impl Toggle for No {
    fn try_create() -> Option<Self> {
        None
    }

    type Or<T: Toggle> = T;
    type And<T: Toggle> = No;
}

const fn mut_unit<'a>() -> &'a mut () {
    // SAFETY: `UNIT` is a zero-sized type (ZST), so its pointer cannot dangle.
    // https://doc.rust-lang.org/reference/behavior-considered-undefined.html#r-undefined.dangling.zero-size
    unsafe { core::ptr::NonNull::dangling().as_mut() }
}

/// Context trait for pull-based streams, allowing operators to be generic over
/// synchronous (`()`) and asynchronous ([`core::task::Context`]) execution contexts.
#[sealed]
pub trait Context<'ctx>: Sized {
    /// The merged context type when combining two pulls.
    type Merged<Other: Context<'ctx>>: Context<'ctx>;

    /// Creates a context reference from a [`core::task::Context`].
    fn from_task<'s>(task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self;

    /// Extracts the self-side context from a merged context.
    fn unmerge_self<'s, Other: Context<'ctx>>(merged: &'s mut Self::Merged<Other>) -> &'s mut Self;
    /// Extracts the other-side context from a merged context.
    fn unmerge_other<'s, Other: Context<'ctx>>(
        merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Other;
}

#[sealed]
impl<'ctx> Context<'ctx> for () {
    type Merged<Other: Context<'ctx>> = Other;

    fn from_task<'s>(_task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self {
        mut_unit()
    }

    fn unmerge_self<'s, Other: Context<'ctx>>(
        _merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Self {
        mut_unit()
    }
    fn unmerge_other<'s, Other: Context<'ctx>>(
        merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Other {
        merged
    }
}

#[sealed]
impl<'ctx> Context<'ctx> for core::task::Context<'ctx> {
    type Merged<Other: Context<'ctx>> = core::task::Context<'ctx>;

    fn from_task<'s>(task_ctx: &'s mut core::task::Context<'ctx>) -> &'s mut Self {
        task_ctx
    }

    fn unmerge_self<'s, Other: Context<'ctx>>(merged: &'s mut Self::Merged<Other>) -> &'s mut Self {
        merged
    }
    fn unmerge_other<'s, Other: Context<'ctx>>(
        merged: &'s mut Self::Merged<Other>,
    ) -> &'s mut Other {
        Other::from_task(merged)
    }
}
