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
//! See the [Hydro docs](https://hydro.run/docs/hydro/reference/correctness/nondet) for more.

/// A non-determinism guard, which documents how a source of non-determinism affects the application.
///
/// To create a non-determinism guard, use the [`nondet!`] macro, which takes in a doc comment
/// explaining the effects of the particular source of non-determinism, and additional
/// non-determinism guards that justify the form of non-determinism.
///
/// The `H` type parameter is the **simulator hook payload** the guard can carry (see
/// `hydro_lang::sim::hooks`), and defaults to `()` so that plain `NonDet` means what it
/// always did. A hook lets a simulation test take manual control of the non-deterministic
/// decisions guarded by this value:
///
/// - An unsafe *operator* (like `batch`) takes a guard with an optional handle for that
///   one operator, e.g. `NonDet<Option<BatchHook<T>>>`. A component that declares a
///   parameter of exactly this type passes it **directly** to the operator it controls,
///   documenting it in a `# Non-Determinism` section of its Rustdoc.
/// - A *component* that contains several unsafe operators can expose them all through one
///   guard by using a tuple payload, e.g. `NonDet<(Option<BatchHook<T>>,
///   Option<SnapshotHook<S>>)>`, letting the test hook some, all, or none of them. The
///   component splits the payload with [`NonDet::take_hook`] and attaches each part to
///   the operator it controls via `nondet!(... hook = part)`.
///
/// Payload types implement [`Default`] ("no hook"), which is what `nondet!(...)` without
/// a `hook =` argument produces.
#[derive(Copy, Clone)]
pub struct NonDet<H = ()> {
    hook: H,
}

impl<H> NonDet<H> {
    /// Creates a guard with no hook attached. Use the [`nondet!`] macro instead of calling
    /// this directly, so that the reason for the non-determinism is documented.
    #[doc(hidden)]
    pub fn unhooked() -> Self
    where
        H: Default,
    {
        NonDet { hook: H::default() }
    }

    /// Creates a guard carrying the given hook payload. Use the [`nondet!`] macro's
    /// `hook = ...` argument instead of calling this directly.
    #[doc(hidden)]
    pub fn hooked(hook: impl Into<H>) -> Self {
        NonDet { hook: hook.into() }
    }

    /// Takes the hook payload out of this guard, leaving the default ("no hook") payload
    /// in place.
    ///
    /// This is how a component splits a *composite* guard across the unsafe operators it
    /// contains: take the payload once, destructure it, and attach each part to the
    /// operator it controls via `nondet!(... hook = part)`. (A guard whose type already
    /// matches a single operator's parameter is simply passed to that operator directly.)
    /// Taking the payload (rather than copying it) ensures a hook is only ever bound where
    /// the binding is visible — re-wrapping a guard with `nondet!` never propagates a
    /// binding.
    pub fn take_hook(&mut self) -> H
    where
        H: Default,
    {
        std::mem::take(&mut self.hook)
    }
}

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
/// An optional trailing `hook = ...` argument attaches a **simulator hook payload** to
/// the guard (see `hydro_lang::sim::hooks`), letting a simulation test script the
/// decisions of the unsafe operator(s) that consume the guard. The expression is
/// converted with [`Into`], so a raw handle can be passed where an optional one is
/// expected:
///
/// ```rust,ignore
/// nondet!(/** reason */)                        // no hook attached (the payload default)
/// nondet!(/// reason
///         nondet_parent)                        // forwarded justification, no hook
/// nondet!(/** reason */ hook = my_hook)         // attach a hook handle
/// nondet!(/** reason */ hook = part)            // attach a payload split off a composite
///                                               // guard with `NonDet::take_hook`
/// nondet!(/** reason */ hook = (h1.into(), None)) // composite payload, hooking only `h1`
/// ```
///
/// Note that forwarding a guard *without* `hook =` never propagates a hook binding, even
/// if the forwarded guard carries one; every binding is visible at the exact
/// operator it controls. A guard whose type already matches an operator's parameter is
/// passed to that operator directly; to attach a hook received as part of a composite
/// payload, split it off explicitly with
/// [`NonDet::take_hook`](crate::nondet::NonDet::take_hook) and pass it via `hook =`.
///
/// # Examples
/// Locally resolved non-determinism:
/// ```rust,no_run
/// # use hydro_lang::prelude::*;
/// use std::time::Duration;
///
/// # #[cfg(feature = "tokio")]
/// fn singleton_with_delay<T, L>(
///   singleton: Singleton<T, Process<L>, Unbounded>
/// ) -> Optional<T, Process<L>, InitNone> {
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
/// use hydro_lang::live_collections::stream::ExactlyOnce;
///
/// use std::fmt::Debug;
/// use std::time::Duration;
///
/// /// ...
/// ///
/// /// # Non-Determinism
/// /// - `nondet_samples`: this function will non-deterministically print elements
/// ///   from the stream according to a timer
/// # #[cfg(feature = "tokio")]
/// fn print_samples<T: Debug, L>(
///   stream: Stream<T, Process<L>, Unbounded>,
///   nondet_samples: NonDet
/// ) {
///   stream
///     .sample_every(q!(Duration::from_secs(1)), nondet!(
///       /// non-deterministic timing will result in non-determistic samples printed
///       nondet_samples
///     ))
///     .assume_retries::<ExactlyOnce>(nondet!(
///         /// non-deterministic duplicated logs are okay
///         nondet_samples
///     ))
///     .for_each(q!(|v| println!("Sample: {:?}", v)))
/// }
/// ```
macro_rules! __nondet__ {
    ($(#[doc = $doc:expr])+ hook = $hook:expr $(,)?) => {
        $crate::nondet::NonDet::hooked($hook)
    };
    ($(#[doc = $doc:expr])+$($forward:ident),*) => {
        {
            $(let _ = $forward;)*
            $crate::nondet::NonDet::unhooked()
        }
    };
}
