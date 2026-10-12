//! Runtime helpers for the embedded deployment backend.
//!
//! This module is NOT gated on the `build` feature so that the staged
//! re-exports are available at runtime in the generated code.

use std::marker::PhantomData;

use futures::Stream;
use stageleft::{QuotedWithContext, RuntimeData, q};

use crate::live_collections::boundedness::{
    Bounded, Boundedness, IsBounded, Unbounded, WeakerBoundednessThan,
};
use crate::live_collections::stream::{
    AtLeastOnce, ExactlyOnce, IsExactlyOnce, IsOrdered, NoOrder, Ordering, Retries, TotalOrder,
    WeakerOrderingThan, WeakerRetryThan,
};
use crate::location::MembershipEvent;
use crate::location::member_id::TaglessMemberId;
use crate::properties::ManualProof;

/// Returns a [`QuotedWithContext`] that references the `__cluster_self_id` runtime variable.
pub fn embedded_cluster_self_id<'a>() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a
{
    let self_id: RuntimeData<&TaglessMemberId> = RuntimeData::new("__cluster_self_id");
    q!(self_id.clone())
}

/// Returns a [`QuotedWithContext`] that references a `__membership_{idx}` runtime variable.
pub fn embedded_cluster_membership_stream<'a>(
    idx: usize,
) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
{
    // TODO(shadaj): change `Deploy` trait to use `syn::Expr` to avoid leaking here
    // TODO(shadaj): this will not work if the same location reads the same membership stream multiple times
    let var_name: &'static str = Box::leak(format!("__membership_{}", idx).into_boxed_str());
    let membership: RuntimeData<
        Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>,
    > = RuntimeData::new(var_name);
    q!(membership)
}

/// A channel crossing the boundary of an embedded dataflow: a network channel end, a stream
/// input, a membership stream, or an output, with its guarantees as Hydro's own live-collection
/// marker types. For a channel flowing into the dataflow (a stream input, a
/// membership stream, or the receiving end of a network channel) the guarantees are what the
/// dataflow relies on, which the host's [`Promises`] must meet; for a channel flowing out of it
/// (an output, or the sending end of a network channel) they are what the dataflow provides,
/// which the host's [`OutputChannel`] requirements must not exceed. A network channel's contract is
/// thus promised once, at its receiving end, covering the host's whole transport.
///
/// Each location's generated module has a zero-sized marker type implementing this for each
/// channel crossing it, named after its field: under `incoming` / `outgoing` for
/// network channel ends, `inputs` for stream inputs, `membership` for membership streams, and
/// `outputs` for outputs. Each field of the generated structs is an [`InputChannel`] (flowing
/// into the dataflow) or an [`OutputChannel`] (flowing out of it) over its marker: directions are
/// named from the dataflow's point of view, as with `EmbeddedInputs` and `EmbeddedOutputs`. The
/// marker is inferred from the field, so the host never names it.
pub trait EmbeddedChannel {
    /// The boundedness the host must keep (for an output, the one the dataflow provides):
    /// `Bounded` (the stream is complete from the start, every element immediately available) or
    /// `Unbounded` (elements may still arrive). Always `Unbounded` for network channels and
    /// membership streams.
    type Boundedness: Boundedness;

    /// The ordering the host must keep (for an output, the ordering the dataflow provides):
    /// `TotalOrder` or `NoOrder`. For a network channel, `TotalOrder` means each sender's
    /// messages in the order it sent them; for an input, that the stream's order is the order
    /// to process; for a membership stream, each member's events in the order they happened.
    type Ordering: Ordering;

    /// The retry guarantee the host must keep (for an output, the one the dataflow provides):
    /// `ExactlyOnce` (nothing delivered twice) or `AtLeastOnce`. For a network channel,
    /// `ExactlyOnce` when the dataflow sends an exactly-once stream on it, which the network
    /// passes on unchanged.
    type Retries: Retries;
}

/// What a host promises about something it supplies, each guarantee promised separately with
/// a [`manual_proof!`](crate::prelude::manual_proof) saying why it holds. An [`InputChannel`] is
/// built from a stream or sink and the `Promises` that meet its marker's guarantees.
pub struct Promises<B, O, R> {
    _promised: PhantomData<fn() -> (B, O, R)>,
}

impl Promises<Unbounded, NoOrder, AtLeastOnce> {
    /// Promises nothing: elements may still be arriving, in any order, and any may be delivered
    /// more than once.
    pub fn new() -> Self {
        Promises {
            _promised: PhantomData,
        }
    }
}

impl Default for Promises<Unbounded, NoOrder, AtLeastOnce> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, O, R> Promises<B, O, R> {
    fn retag<B2, O2, R2>(self) -> Promises<B2, O2, R2> {
        Promises {
            _promised: PhantomData,
        }
    }
}

impl<O, R> Promises<Unbounded, O, R> {
    /// Promises that the stream is complete from the start, as a Hydro `Bounded` collection is:
    /// every element is immediately available, and the stream ends without ever waiting (for
    /// example `futures::stream::iter` over a `Vec`). Being finite is not enough: a stream whose
    /// elements arrive over time is unbounded even if it ends.
    pub fn bounded(self, _proof: ManualProof) -> Promises<Bounded, O, R> {
        self.retag()
    }
}

impl<B, R> Promises<B, NoOrder, R> {
    /// Promises the order: for a channel, each sender's messages are delivered in the order it
    /// sent them; for an input, the stream's order is the order to process; for a membership
    /// stream, each member's events arrive in the order they happened.
    pub fn total_order(self, _proof: ManualProof) -> Promises<B, TotalOrder, R> {
        self.retag()
    }
}

impl<B, O> Promises<B, O, AtLeastOnce> {
    /// Promises that nothing is delivered twice: a channel never duplicates a message (for
    /// example by resending it after a reconnect), and every element of an input is a distinct
    /// event.
    pub fn exactly_once(self, _proof: ManualProof) -> Promises<B, O, ExactlyOnce> {
        self.retag()
    }
}

/// A channel whose guarantees promises of boundedness `B`, ordering `O`, and retries `R` meet:
/// each guarantee the channel needs is weaker than (or the same as) the one promised, by Hydro's
/// own [`WeakerBoundednessThan`], [`WeakerOrderingThan`], and [`WeakerRetryThan`] relations. It
/// is what [`InputChannel::new`] requires, and lets a reusable transport state once what it
/// provides: `fn source<C: SatisfiedBy<Unbounded, TotalOrder, ExactlyOnce>>()` serves every
/// channel that needs at most that.
pub trait SatisfiedBy<B, O, R>: EmbeddedChannel {}

impl<K, B, O, R> SatisfiedBy<B, O, R> for K
where
    K: EmbeddedChannel,
    K::Boundedness: WeakerBoundednessThan<B>,
    K::Ordering: WeakerOrderingThan<O>,
    K::Retries: WeakerRetryThan<R>,
{
}

/// A channel flowing into an embedded dataflow, as the host provides it: a `Stream` for a stream
/// input, a membership stream, or the receiving end of a network channel, for the channel `C`
/// (its generated marker type, see [`EmbeddedChannel`]).
///
/// An `InputChannel` exists only with promises that meet `C`'s guarantees:
/// [`InputChannel::new`] takes the [`Promises`] and checks them, so wiring a channel always
/// states its contract and a mismatch with the dataflow is a type error at that call. Promising
/// more than a channel needs is fine. `C` is inferred from the field the `InputChannel` fills.
pub struct InputChannel<C, S> {
    inner: S,
    _channel: PhantomData<fn() -> C>,
}

impl<C: EmbeddedChannel, S> InputChannel<C, S> {
    /// Wraps a host stream for channel `C`, with `promises` that must meet its guarantees.
    pub fn new<B, O, R>(inner: S, promises: Promises<B, O, R>) -> Self
    where
        C: SatisfiedBy<B, O, R>,
    {
        let _ = promises;
        InputChannel {
            inner,
            _channel: PhantomData,
        }
    }
}

impl<C, S> InputChannel<C, S> {
    /// The host's stream. Used by generated code.
    #[doc(hidden)]
    pub fn into_inner(self) -> S {
        self.inner
    }
}

/// A channel flowing out of an embedded dataflow, as the host provides it: an `FnMut` sink for an
/// output or the sending end of a network channel, for the channel `M` (its generated marker
/// type, under `outputs` or `outgoing`), whose [`EmbeddedChannel`] guarantees are what the
/// dataflow's stream provides.
///
/// Here the dataflow provides the guarantees and the host relies on them.
/// [`OutputChannel::new`] requires nothing; a host whose sink depends on a guarantee says so
/// with [`OutputChannel::require_bounded`], [`OutputChannel::require_total_order`], or
/// [`OutputChannel::require_exactly_once`], each of which compiles only if `M`'s contract
/// provides it. `M` is inferred from the field the
/// `OutputChannel` fills.
pub struct OutputChannel<M, S> {
    inner: S,
    _output: PhantomData<fn() -> M>,
}

impl<M: EmbeddedChannel, S> OutputChannel<M, S> {
    /// Wraps a host sink that requires nothing of the elements it receives: they may keep
    /// arriving, in any order, and an element may arrive more than once.
    pub fn new(inner: S) -> Self {
        OutputChannel {
            inner,
            _output: PhantomData,
        }
    }

    /// Requires that the dataflow's output stream be complete from the start, every element
    /// produced at once. Compiles only if the dataflow's output stream is bounded.
    pub fn require_bounded(self) -> Self
    where
        M::Boundedness: IsBounded,
    {
        self
    }

    /// Requires that the sink receive elements in the dataflow's order. Compiles only if the
    /// dataflow's output stream is totally ordered.
    pub fn require_total_order(self) -> Self
    where
        M::Ordering: IsOrdered,
    {
        self
    }

    /// Requires that the sink receive each element exactly once. Compiles only if the dataflow's
    /// output stream is exactly-once.
    pub fn require_exactly_once(self) -> Self
    where
        M::Retries: IsExactlyOnce,
    {
        self
    }
}

impl<M, S> OutputChannel<M, S> {
    /// The host's sink. Used by generated code.
    #[doc(hidden)]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}
