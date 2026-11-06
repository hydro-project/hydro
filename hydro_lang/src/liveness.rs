//! Defines the `NonLive` type and `nonlive!` macro for tracking liveness concerns.
//!
//! Hydro's safe APIs guarantee determinism: every output is a realization of the output the
//! program would have if every machine stayed live and every message arrived (its *ideal*). They
//! do not guarantee *liveness*, that a realization eventually reaches the ideal. Where a program
//! relies on it, for example by treating a stream that may be missing elements as one that is
//! not ([`MayDrop`](crate::live_collections::stream::MayDrop) to
//! [`NoDrop`](crate::live_collections::stream::NoDrop)), it passes a **liveness guard**, with type
//! `NonLive`, by invoking `nonlive!()` with an explanation of why the program is acceptable even
//! though some output there may never materialize: the concern is mitigated elsewhere (for
//! example by retransmission), or the application tolerates it.
//!
//! `nonlive!` is to liveness what [`nondet!`](crate::nondet::nondet) is to safety: it has no
//! runtime effect, and it marks the point where the program takes on the concern.

/// A liveness guard, which documents why a program is acceptable at a point where some output
/// may never materialize.
///
/// To create a liveness guard, use the [`nonlive!`] macro, which takes in a doc comment
/// explaining why the liveness concern is acceptable, and additional liveness guards that
/// justify it. A component whose progress depends on something it cannot check takes a `NonLive`
/// parameter and documents, under a `# Liveness` heading of its Rustdoc, what the caller must
/// explain.
#[derive(Copy, Clone)]
pub struct NonLive {
    _private: (),
}

impl NonLive {
    /// Creates a liveness guard. Use the [`nonlive!`] macro instead of calling this directly, so
    /// that the reason for the liveness concern is documented.
    #[doc(hidden)]
    pub fn justified() -> Self {
        NonLive { _private: () }
    }
}

#[doc(inline)]
pub use crate::__nonlive__ as nonlive;

#[macro_export]
/// Fulfills a liveness guard parameter by declaring why the program is acceptable even though
/// some output may never materialize, or by providing other liveness guards that answer for it.
///
/// The first argument must be a doc comment with the explanation. If forwarding a parent guard,
/// because the concern is not resolved locally, explain how the inner concern is answered for by
/// the outer one.
///
/// ```rust,ignore
/// nonlive!(/** every event is retransmitted until it is acknowledged */)
/// nonlive!(/// an acceptor whose connection fails is counted among the f that may fail
///          acceptor_liveness)
/// ```
macro_rules! __nonlive__ {
    ($(#[doc = $doc:expr])+$($forward:ident),*) => {
        {
            $(let _ = $forward;)*
            $crate::liveness::NonLive::justified()
        }
    };
}
