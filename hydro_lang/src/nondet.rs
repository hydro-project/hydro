//! Defines the `NonDet` type and `nondet!` macro for tracking non-determinism.
//!
//! All **safe** APIs in Hydro guarantee determinism, even in the face of networking delays
//! and concurrency across machines. But often it is necessary to do something non-deterministic,
//! like generate events at a fixed wall-clock-time interval, or split an input into arbitrarily
//! sized batches.
//!
//! These non-deterministic APIs take additional parameters called **non-determinism guards**.
//! These values, with type `NonDet`, help you reason about how non-determinism affects your
//! application. To pass a non-determinism guard, you must invoke `nondet!()` with an explanation
//! for how the non-determinism affects the application.
//!
//! See the [Hydro docs](https://hydro.run/docs/hydro/live-collections/determinism) for more.

/// A non-determinism guard, which documents how a source of non-determinism affects the application.
///
/// To create a non-determinism guard, use the [`nondet!`] macro, which takes in a doc comment
/// explaining the effects of the particular source of non-determinism, and additional
/// non-determinism guards that justify the form of non-determinism.
#[derive(Copy, Clone)]
pub struct NonDet;

#[doc(inline)]
pub use crate::__nondet__ as nondet;

#[macro_export]
/// Fulfills a non-determinism guard parameter by declaring a reason why the
/// non-determinism is tolerated or providing other non-determinism guards
/// that forward the inner non-determinism.
///
/// The first argument must be a doc comment with the reason the non-determinism
/// is okay. If forwarding a parent non-determinism, because the non-determinism
/// is not handled internally, you should provide a short explanation of how the
/// inner non-determinism is captured by the outer one. If the non-determinism
/// is locally resolved, you should document _why_ this is the case.
///
/// # Examples
/// Locally resolved non-determinism:
/// ```rust,no_run
/// # use hydro_lang::prelude::*;
/// use std::time::Duration;
///
/// fn singleton_with_delay<T, L>(
///   singleton: Singleton<T, Process<L>, Unbounded>
/// ) -> Optional<T, Process<L>, Unbounded> {
///   singleton
///     .sample_every(q!(Duration::from_secs(1)), nondet!(/**
///         non-deterministic samples will eventually resolve to stable result
///     */))
///     .last()
/// }
/// ```
///
/// Forwarded non-determinism:
/// ```rust
/// # use hydro_lang::prelude::*;
/// use std::fmt::Debug;
/// use std::time::Duration;
///
/// /// ...
/// ///
/// /// # Non-Determinism
/// /// - `nondet_samples`: this function will non-deterministically print elements
/// ///   from the stream according to a timer
/// fn print_samples<T: Debug, L>(
///   stream: Stream<T, Process<L>, Unbounded>,
///   nondet_samples: NonDet
/// ) {
///   stream
///     .sample_every(q!(Duration::from_secs(1)), nondet!(
///       /// non-deterministic timing will result in non-determistic samples printed
///       nondet_samples
///     ))
///     .for_each(q!(|v| println!("Sample: {:?}", v)))
/// }
/// ```
macro_rules! __nondet__ {
    ($(#[doc = $doc:expr])+$($forward:ident),*) => {
        {
            $(let _ = $forward;)*
            $crate::nondet::NonDet
        }
    };
}
