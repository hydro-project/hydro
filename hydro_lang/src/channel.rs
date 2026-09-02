//! Channels for wiring up live collections, used for forward references and cycles in Hydro.

use std::cell::Cell;

use sealed::sealed;

use crate::compile::builder::{CycleId, RootId};
use crate::compile::ir::CollectionKind;
#[cfg(stageleft_runtime)]
use crate::compile::ir::HydroNode;
use crate::location::Location;
use crate::location::dynamic::LocationId;
use crate::staging_util::Invariant;

#[sealed]
pub(crate) trait ReceiverKind {
    /// The token used to identify the receiving end when completing it with a collection.
    type CompletionToken: Clone;
}

/// Marks that the channel is a "forward reference" to a later-defined collection.
///
/// When data is sent into the channel, the provided collection must not depend
/// _synchronously_ (in the same tick) on the receiving end of the channel that
/// was created earlier.
pub enum ForwardRef {}

#[sealed]
impl ReceiverKind for ForwardRef {
    /// Channels register their [`crate::compile::ir::HydroRoot::CycleSink`] when they are
    /// created, so completion fills in the input of that existing root.
    type CompletionToken = RootId;
}

/// Marks that the [`TickCycleHandle`] will send a live collection to the next tick.
///
/// Dependency cycles are permitted for this handle type, because the collection used
/// to complete this handle will appear on the source-side on the _next_ tick.
pub enum TickCycle {}

#[sealed]
impl ReceiverKind for TickCycle {
    /// Tick cycles register their [`crate::compile::ir::HydroRoot::CycleSink`] when they
    /// are completed, so completion only needs the cycle ID.
    type CompletionToken = CycleId;
}

pub(crate) trait ReceiverComplete<'a, Marker>
where
    Marker: ReceiverKind,
{
    fn complete(self, token: Marker::CompletionToken, expected_location: LocationId);
}

pub(crate) trait CycleCollection<'a, Kind>: ReceiverComplete<'a, Kind>
where
    Kind: ReceiverKind,
{
    type Location: Location<'a>;

    fn collection_kind() -> CollectionKind;

    fn create_source(id: CycleId, location: Self::Location) -> Self;
}

/// Implemented by live collections that can be the receiving half of a channel created
/// with [`Location::channel`], determining the type of the sending half.
///
/// Most collections use [`ChannelSender`], which permits the channel to act as a null
/// input if the sender is dropped without sending. Collections that must always have a
/// value (such as [`crate::live_collections::singleton::Singleton`]) instead use
/// [`RequiredChannelSender`], which panics if dropped without sending.
///
/// This trait cannot be usefully implemented outside of Hydro, since creating a sender
/// (via the crate-private `CreateChannelSender` trait) is only possible for Hydro's own
/// live collections.
pub trait ChannelTarget<'a> {
    /// The type of the sending half of a channel targeting this collection.
    type Sender;
}

pub(crate) trait CreateChannelSender<'a>: ChannelTarget<'a> {
    fn create_sender(sink_id: RootId, expected_location: LocationId) -> Self::Sender;
}

pub(crate) trait CycleCollectionWithInitial<'a, Kind>: ReceiverComplete<'a, Kind>
where
    Kind: ReceiverKind,
{
    type Location: Location<'a>;

    fn location(&self) -> &Self::Location;

    fn create_source_with_initial(
        cycle_id: CycleId,
        initial: Self,
        location: Self::Location,
    ) -> Self;
}

/// The sending half of a channel created with [`Location::channel`], which is used to
/// wire up a collection as the source for the receiving half created earlier.
///
/// The `C` type parameter specifies the collection type that can be sent into the channel.
/// Depending on the type of the target collection, `send` will either consume the sender
/// (when the target is ordered, such as a [`crate::live_collections::stream::TotalOrder`]
/// stream, since only a single collection can provide the elements without ordering
/// ambiguity) or take it by reference (when the target is unordered, in which case the
/// sent collections are merged as unordered streams).
///
/// If the sender is dropped without any data being sent, the receiving end of the channel
/// acts as a null input and never receives any elements.
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
pub struct ChannelSender<'a, C: ReceiverComplete<'a, ForwardRef>> {
    sent: Cell<bool>,
    sink_id: RootId,
    expected_location: LocationId,
    _phantom: Invariant<'a, C>,
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, ForwardRef>> ChannelSender<'a, C> {
    pub(crate) fn new(sink_id: RootId, expected_location: LocationId) -> Self {
        Self {
            sent: Cell::new(false),
            sink_id,
            expected_location,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sends the given collection into the channel, consuming the sender so that no
    /// further collections can be sent.
    pub(crate) fn send_exclusive(self, value: C) {
        C::complete(value, self.sink_id, self.expected_location)
    }

    /// Sends a collection into the channel, merging it with any previously sent
    /// collections by extending the channel's cycle sink with a [`HydroNode::Chain`].
    ///
    /// This is only safe when the target collection is unordered, since the order in
    /// which the sent collections are interleaved is non-deterministic.
    #[cfg(stageleft_runtime)]
    pub(crate) fn send_merge<L: Location<'a>>(
        &self,
        location: &L,
        value_node: HydroNode,
        collection_kind: CollectionKind,
    ) {
        assert_eq!(
            Location::id(location),
            self.expected_location,
            "locations do not match"
        );

        let flow_state = location.flow_state().clone();
        let mut flow_state = flow_state.borrow_mut();
        let sink_input = flow_state.cycle_sink_input_mut(self.sink_id);
        if self.sent.get() {
            let prev = std::mem::replace(sink_input, HydroNode::Placeholder);
            *sink_input = HydroNode::Chain {
                first: Box::new(prev),
                second: Box::new(value_node),
                metadata: location.new_node_metadata(collection_kind),
            };
        } else {
            // The first send replaces the null input the channel was created with.
            *sink_input = value_node;
        }

        self.sent.set(true);
    }
}

/// The sending half of a channel created with [`Location::channel`] whose target
/// collection **must** be sent a value, used to wire up a collection as the source for
/// the receiving half created earlier.
///
/// This sender is used instead of [`ChannelSender`] when the target collection cannot
/// act as a null input. In particular, a [`crate::live_collections::singleton::Singleton`]
/// must always have a value, so a channel targeting one cannot be left empty.
///
/// # Panics
/// Panics if dropped without [`send`](RequiredChannelSender::send) being called.
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
pub struct RequiredChannelSender<'a, C: ReceiverComplete<'a, ForwardRef>> {
    completed: bool,
    sink_id: RootId,
    expected_location: LocationId,
    _phantom: Invariant<'a, C>,
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, ForwardRef>> RequiredChannelSender<'a, C> {
    pub(crate) fn new(sink_id: RootId, expected_location: LocationId) -> Self {
        Self {
            completed: false,
            sink_id,
            expected_location,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Sends the given collection into the channel, consuming the sender so that no
    /// further collections can be sent.
    pub(crate) fn send_exclusive(mut self, value: C) {
        self.completed = true;
        C::complete(value, self.sink_id, self.expected_location.clone())
    }
}

impl<'a, C: ReceiverComplete<'a, ForwardRef>> Drop for RequiredChannelSender<'a, C> {
    fn drop(&mut self) {
        if !self.completed && !std::thread::panicking() {
            panic!(
                "channel sender dropped without sending a value, but the target collection (such as a `Singleton`) must always have a value and cannot be left empty"
            );
        }
    }
}

/// A handle that can be used to complete a tick cycle by sending a collection to the next tick.
///
/// The `C` type parameter specifies the collection type that can be used to complete the handle.
#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
pub struct TickCycleHandle<'a, C: ReceiverComplete<'a, TickCycle>> {
    completed: bool,
    cycle_id: CycleId,
    expected_location: LocationId,
    _phantom: Invariant<'a, C>,
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, TickCycle>> TickCycleHandle<'a, C> {
    pub(crate) fn new(cycle_id: CycleId, expected_location: LocationId) -> Self {
        Self {
            completed: false,
            cycle_id,
            expected_location,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, C: ReceiverComplete<'a, TickCycle>> Drop for TickCycleHandle<'a, C> {
    fn drop(&mut self) {
        if !self.completed && !std::thread::panicking() {
            panic!("TickCycleHandle dropped without being completed");
        }
    }
}

#[expect(
    private_bounds,
    reason = "only Hydro collections can implement ReceiverComplete"
)]
impl<'a, C: ReceiverComplete<'a, TickCycle>> TickCycleHandle<'a, C> {
    /// Sends the provided collection to the next tick, where it will be materialized
    /// in the collection returned by [`crate::location::Tick::cycle`] or
    /// [`crate::location::Tick::cycle_with_initial`].
    pub fn complete_next_tick(mut self, stream: impl Into<C>) {
        self.completed = true;
        C::complete(stream.into(), self.cycle_id, self.expected_location.clone())
    }
}

/// A trait for completing a cycle handle with a state value.
/// Used internally by the `sliced!` macro for state management.
#[doc(hidden)]
pub trait CompleteCycle<S> {
    /// Completes the cycle with the given state value.
    fn complete_next_tick(self, state: S);
}

impl<'a, C: ReceiverComplete<'a, TickCycle>> CompleteCycle<C> for TickCycleHandle<'a, C> {
    fn complete_next_tick(self, state: C) {
        TickCycleHandle::complete_next_tick(self, state)
    }
}
