//! Runtime helpers for the embedded deployment backend.
//!
//! This module is NOT gated on the `build` feature so that the staged
//! re-exports are available at runtime in the generated code.

use std::marker::PhantomData;

use futures::Stream;
use stageleft::{QuotedWithContext, RuntimeData, q};

use crate::live_collections::stream::{
    AtLeastOnce, ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder, WeakerOrderingThan,
    WeakerRetryThan,
};
use crate::location::MembershipEvent;
use crate::location::member_id::TaglessMemberId;
use crate::networking::NetworkingConfig;
use crate::nondet::NonDet;
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
/// input, a membership stream, or an output, with its guarantees as marker types (`Ordering`
/// and `Retries`).
///
/// - For a stream input or membership stream, the guarantees are what the dataflow relies on,
///   which the host's [`Promises`] must meet.
/// - For the sending end of a network channel, they are what the dataflow sends, which the
///   host's [`Acknowledgements`] must cover, as for an output.
/// - For the receiving end of a network channel, they are what the dataflow relies on: the sent
///   stream, weakened by the channel's configuration. The host's [`Promises`] must meet them, as
///   for a stream input.
/// - For an output, they are what the dataflow provides. The host's [`Acknowledgements`] must
///   cover every way the output may be weaker than ordered and exactly-once.
///
/// Each location's generated module has a marker type implementing this for each
/// channel crossing it, named after its field: under `incoming` / `outgoing` for
/// network channel ends, `inputs` for stream inputs, `membership` for membership streams, and
/// `outputs` for outputs. Each field of the generated structs is an [`InputChannel`] (a stream
/// input or membership stream), a [`ReceiveChannel`] (a receiving end), a [`SendChannel`] (a
/// sending end), or an [`OutputChannel`] (an output) over its marker. The marker is inferred from
/// the field. The markers of network channel ends also implement [`EmbeddedNetworkChannel`].
pub trait EmbeddedChannel {
    /// The ordering the host must keep (for an output or the sending end of a network channel,
    /// the ordering the dataflow provides): `TotalOrder` or `NoOrder`. For a network channel,
    /// `TotalOrder` means each sender's messages in the order it sent them; for an input, that
    /// the stream's order is the order to process; for a membership stream, each member's events
    /// in the order they happened.
    type Ordering: Ordering;

    /// The retry guarantee the host must keep (for an output or the sending end of a network
    /// channel, the one the dataflow provides): `ExactlyOnce` (nothing delivered twice) or
    /// `AtLeastOnce`.
    type Retries: Retries;
}

/// What a host promises about something it supplies, each guarantee promised separately with
/// a [`manual_proof!`](crate::prelude::manual_proof) saying why it holds. An [`InputChannel`] or
/// [`ReceiveChannel`] is built from a stream and the `Promises` that meet its marker's
/// guarantees.
pub struct Promises<O, R> {
    _promised: PhantomData<fn() -> (O, R)>,
}

impl Promises<NoOrder, AtLeastOnce> {
    /// Promises nothing: elements may arrive in any order, and any may be delivered more than
    /// once.
    pub fn new() -> Self {
        Promises {
            _promised: PhantomData,
        }
    }
}

impl Default for Promises<NoOrder, AtLeastOnce> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O, R> Promises<O, R> {
    fn retag<O2, R2>(self) -> Promises<O2, R2> {
        Promises {
            _promised: PhantomData,
        }
    }
}

impl<R> Promises<NoOrder, R> {
    /// Promises the order: for a channel, each sender's messages are delivered in the order it
    /// sent them; for an input, the stream's order is the order to process; for a membership
    /// stream, each member's events arrive in the order they happened.
    pub fn total_order(self, _proof: ManualProof) -> Promises<TotalOrder, R> {
        self.retag()
    }
}

impl<O> Promises<O, AtLeastOnce> {
    /// Promises that nothing is delivered twice: a channel never duplicates a message (for
    /// example by resending it after a reconnect), and every element of an input is a distinct
    /// event.
    pub fn exactly_once(self, _proof: ManualProof) -> Promises<O, ExactlyOnce> {
        self.retag()
    }
}

/// A channel whose guarantees promises of ordering `O` and retries `R` meet: each guarantee the
/// channel needs is weaker than (or the same as) the one promised, by the
/// [`WeakerOrderingThan`] and [`WeakerRetryThan`] relations. It is what [`InputChannel::new`]
/// requires, and lets a reusable transport state once what it provides:
/// `fn source<C: SatisfiedBy<TotalOrder, ExactlyOnce>>()` serves every channel that needs at
/// most that.
pub trait SatisfiedBy<O, R>: EmbeddedChannel {}

impl<K, O, R> SatisfiedBy<O, R> for K
where
    K: EmbeddedChannel,
    K::Ordering: WeakerOrderingThan<O>,
    K::Retries: WeakerRetryThan<R>,
{
}

/// A channel flowing into an embedded dataflow, as the host provides it: a `Stream` for a stream
/// input or a membership stream, for the channel `C` (its generated marker type, see
/// [`EmbeddedChannel`]). The receiving end of a network channel is a [`ReceiveChannel`] instead.
///
/// An `InputChannel` exists only with promises that meet `C`'s guarantees:
/// [`InputChannel::new`] takes the [`Promises`] and checks them, so wiring a channel always
/// states its guarantees and a mismatch with the dataflow is a type error at that call. Promising
/// more than a channel needs is fine. `C` is inferred from the field the `InputChannel` fills.
pub struct InputChannel<C, S> {
    inner: S,
    _channel: PhantomData<fn() -> C>,
}

impl<C: EmbeddedChannel, S> InputChannel<C, S> {
    /// Wraps a host stream for channel `C`, with `promises` that must meet its guarantees.
    pub fn new<O, R>(inner: S, promises: Promises<O, R>) -> Self
    where
        C: SatisfiedBy<O, R>,
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

/// The configuration of a network channel crossing the boundary of an embedded dataflow: the
/// transport type `Transport` of the network config the dataflow sends with (for example
/// `Tcp<FailStop>` for `TCP.fail_stop()`, or `EmbeddedTransport<ExactlyOnce, NoOrder, FailStop>`
/// for `EMBEDDED.may_reorder()`), and its serialization `Serialization` (`Bincode`, or `Embedded`
/// for an unserialized `EMBEDDED` channel).
///
/// The markers of both ends of a network channel implement it, alongside [`EmbeddedChannel`]
/// for the stream at that end. The host acknowledges the configuration exactly at each end with
/// a [`ChannelConfiguration`], so that changing the channel's network config fails the host's
/// build at both ends until it acknowledges the change.
pub trait EmbeddedNetworkChannel: EmbeddedChannel {
    /// The transport type of the channel's network config.
    type Transport;

    /// The serialization of the channel's network config.
    type Serialization;
}

/// The configuration of a network channel as the host acknowledges it: the transport `Tr` and
/// serialization `S` of its network config, which must match the channel's exactly (see
/// [`EmbeddedNetworkChannel`]). It is built from the same network config value the dataflow
/// sends with; the config's name is ignored.
pub struct ChannelConfiguration<Tr, S> {
    _acknowledged: PhantomData<fn() -> (Tr, S)>,
}

impl<Tr, S> ChannelConfiguration<Tr, S> {
    /// Acknowledges a channel configured with `config`, the same network config the dataflow
    /// sends with (for example `TCP.fail_stop().bincode()`).
    pub fn new<Name>(config: NetworkingConfig<Tr, S, Name>) -> Self {
        let _ = config;
        ChannelConfiguration {
            _acknowledged: PhantomData,
        }
    }
}

/// A network channel end whose configuration is exactly the acknowledged transport `Tr` and
/// serialization `S`. Each part is checked separately, so a mismatch names the part the host
/// acknowledges wrongly. It is what [`SendChannel::new`] and [`ReceiveChannel::new`] require.
pub trait ConfiguredAs<Tr, S>: EmbeddedNetworkChannel {}

impl<C, Tr, S> ConfiguredAs<Tr, S> for C
where
    C: EmbeddedNetworkChannel,
    Tr: AcknowledgesTransport<C::Transport, C>,
    S: AcknowledgesSerialization<C::Serialization, C>,
{
}

/// An acknowledged transport (`Self`) of the network channel end `End`, which must be the
/// transport `Declared` of the channel's network config exactly.
#[diagnostic::on_unimplemented(
    message = "the channel end `{End}` is configured with `{Declared}`, but the host acknowledges `{Self}`",
    label = "acknowledges `{Self}`",
    note = "acknowledge the same network config the dataflow sends with; the marker's docs give it"
)]
pub trait AcknowledgesTransport<Declared, End> {}

impl<X, End> AcknowledgesTransport<X, End> for X {}

/// An acknowledged serialization (`Self`) of the network channel end `End`, which must be the
/// serialization `Declared` of the channel's network config exactly.
#[diagnostic::on_unimplemented(
    message = "the channel end `{End}` is serialized with `{Declared}`, but the host acknowledges `{Self}`",
    label = "acknowledges `{Self}`",
    note = "acknowledge the same network config the dataflow sends with; the marker's docs give it"
)]
pub trait AcknowledgesSerialization<Declared, End> {}

impl<X, End> AcknowledgesSerialization<X, End> for X {}

/// The sending end of a network channel, as the host provides it: an `FnMut` sink that hands
/// each message to the host's transport, for the channel end `C` (its generated marker type,
/// under `outgoing`).
///
/// The host agrees to both inputs of the dataflow's send: [`SendChannel::new`] takes the
/// [`ChannelConfiguration`] the host acknowledges, which must match the channel's exactly, and the
/// [`Acknowledgements`] of the stream the dataflow sends, which must cover every way it may be
/// weaker than ordered and exactly-once, as for an [`OutputChannel`]. `C` is inferred from the
/// field the `SendChannel` fills.
pub struct SendChannel<C, S> {
    inner: S,
    _channel: PhantomData<fn() -> C>,
}

impl<C: EmbeddedNetworkChannel, S> SendChannel<C, S> {
    /// Wraps a host sink for the channel end `C`, with the acknowledgements of its
    /// configuration (`channel`) and of the stream sent on it (`acknowledgements`).
    pub fn new<Tr, Ser, O, R>(
        inner: S,
        channel: ChannelConfiguration<Tr, Ser>,
        acknowledgements: Acknowledgements<O, R>,
    ) -> Self
    where
        C: ConfiguredAs<Tr, Ser> + AcknowledgedBy<O, R>,
    {
        let _ = (channel, acknowledgements);
        SendChannel {
            inner,
            _channel: PhantomData,
        }
    }
}

impl<C, S> SendChannel<C, S> {
    /// The host's sink. Used by generated code.
    #[doc(hidden)]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}

/// The receiving end of a network channel, as the host provides it: a `Stream` of the messages
/// its transport delivers, for the channel end `C` (its generated marker type, under
/// `incoming`).
///
/// The host agrees to the channel's configuration and to the stream it delivers:
/// [`ReceiveChannel::new`] takes the [`ChannelConfiguration`] the host acknowledges,
/// which must match exactly, and the [`Promises`] of the stream it delivers, which must meet what
/// the dataflow relies on, as for an [`InputChannel`]. That stream is the sent stream weakened by
/// the channel's configuration (a channel that may reorder delivers a `NoOrder` stream), so it
/// may be weaker than what the sending end acknowledges. `C` is inferred from the field the
/// `ReceiveChannel` fills.
pub struct ReceiveChannel<C, S> {
    inner: S,
    _channel: PhantomData<fn() -> C>,
}

impl<C: EmbeddedNetworkChannel, S> ReceiveChannel<C, S> {
    /// Wraps a host stream for the channel end `C`, with the acknowledgement of its
    /// configuration (`channel`) and the promises of the stream it delivers (`promises`).
    pub fn new<Tr, Ser, O, R>(
        inner: S,
        channel: ChannelConfiguration<Tr, Ser>,
        promises: Promises<O, R>,
    ) -> Self
    where
        C: ConfiguredAs<Tr, Ser> + SatisfiedBy<O, R>,
    {
        let _ = (channel, promises);
        ReceiveChannel {
            inner,
            _channel: PhantomData,
        }
    }
}

impl<C, S> ReceiveChannel<C, S> {
    /// The host's stream. Used by generated code.
    #[doc(hidden)]
    pub fn into_inner(self) -> S {
        self.inner
    }
}

/// What a host acknowledges about a stream the dataflow provides (an output, or the stream sent
/// on a network channel): each way it may be weaker than ordered and exactly-once that the
/// host's sink tolerates, with a [`nondet!`](crate::prelude::nondet) saying why. An
/// [`OutputChannel`] or [`SendChannel`] is built from a sink and the `Acknowledgements` that
/// cover what its marker's stream may do.
pub struct Acknowledgements<O, R> {
    _acknowledged: PhantomData<fn() -> (O, R)>,
}

impl Acknowledgements<TotalOrder, ExactlyOnce> {
    /// Acknowledges nothing: the sink relies on receiving elements in the dataflow's order, each
    /// exactly once.
    pub fn new() -> Self {
        Acknowledgements {
            _acknowledged: PhantomData,
        }
    }
}

impl Default for Acknowledgements<TotalOrder, ExactlyOnce> {
    fn default() -> Self {
        Self::new()
    }
}

impl<O, R> Acknowledgements<O, R> {
    fn retag<O2, R2>(self) -> Acknowledgements<O2, R2> {
        Acknowledgements {
            _acknowledged: PhantomData,
        }
    }
}

impl<R> Acknowledgements<TotalOrder, R> {
    /// Acknowledges that elements may arrive in any order.
    pub fn no_order(self, _nondet: NonDet) -> Acknowledgements<NoOrder, R> {
        self.retag()
    }
}

impl<O> Acknowledgements<O, ExactlyOnce> {
    /// Acknowledges that an element may arrive more than once.
    pub fn at_least_once(self, _nondet: NonDet) -> Acknowledgements<O, AtLeastOnce> {
        self.retag()
    }
}

/// An output whose guarantees acknowledgements of ordering `O` and retries `R` cover: each
/// acknowledged guarantee is weaker than (or the same as) the one the output provides, by the
/// [`WeakerOrderingThan`] and [`WeakerRetryThan`] relations. It is what [`OutputChannel::new`]
/// requires.
pub trait AcknowledgedBy<O, R>: EmbeddedChannel {}

impl<K, O, R> AcknowledgedBy<O, R> for K
where
    K: EmbeddedChannel,
    O: WeakerOrderingThan<K::Ordering>,
    R: WeakerRetryThan<K::Retries>,
{
}

/// An output of an embedded dataflow, as the host provides it: an `FnMut` sink for the channel
/// `M` (its generated marker type, under `outputs`), whose [`EmbeddedChannel`] guarantees are
/// what the dataflow's output stream provides.
///
/// An `OutputChannel` exists only with acknowledgements that cover what the output may do:
/// [`OutputChannel::new`] takes the [`Acknowledgements`] and checks them, so a sink cannot
/// silently receive elements out of order or more than once. Acknowledging more than an output
/// may do is fine. `M` is inferred from the field the `OutputChannel` fills.
pub struct OutputChannel<M, S> {
    inner: S,
    _output: PhantomData<fn() -> M>,
}

impl<M: EmbeddedChannel, S> OutputChannel<M, S> {
    /// Wraps a host sink for output `M`, with `acknowledgements` that cover what `M` may do.
    pub fn new<O, R>(inner: S, acknowledgements: Acknowledgements<O, R>) -> Self
    where
        M: AcknowledgedBy<O, R>,
    {
        let _ = acknowledgements;
        OutputChannel {
            inner,
            _output: PhantomData,
        }
    }
}

impl<M, S> OutputChannel<M, S> {
    /// The host's sink. Used by generated code.
    #[doc(hidden)]
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }
}
