//! Organizational module for [`SendCtx`]/[`RecvCtx`] structs and [`SendPort`]/[`RecvPort`] structs.
use std::cell::RefMut;
use std::marker::PhantomData;

use ref_cast::RefCast;
use sealed::sealed;

use super::HandoffId;
use crate::scheduled::graph::Dfir;
use crate::scheduled::handoff::{CanReceive, Handoff, TeeingHandoff, TryCanReceive};

/// An empty trait used to denote [`Polarity`]: either **send** or **receive**.
///
/// [`SendPort`] and [`RecvPort`] have identical representations (via [`Port`]) but are not
/// interchangable, so [`SEND`] and [`RECV`] which implement this trait are used to differentiate
/// between the two polarities.
#[sealed]
pub trait Polarity: 'static {}

/// An uninstantiable type used to tag port [`Polarity`] as **send**.
///
/// See also: [`RECV`].
#[expect(clippy::upper_case_acronyms, reason = "marker type")]
pub enum SEND {}
/// An uninstantiable type used to tag port [`Polarity`] as **receive**.
///
/// See also: [`SEND`].
#[expect(clippy::upper_case_acronyms, reason = "marker type")]
pub enum RECV {}
#[sealed]
impl Polarity for SEND {}
#[sealed]
impl Polarity for RECV {}

/// Lightweight ID struct representing an input or output port for a [`Handoff`] added to a
/// [`Dfir`] instance..
#[must_use]
pub struct Port<S: Polarity, H>
where
    H: Handoff,
{
    pub(crate) handoff_id: HandoffId,
    #[expect(clippy::type_complexity, reason = "phantom data")]
    pub(crate) _marker: PhantomData<(*const S, fn() -> H)>,
}
/// Send-specific variant of [`Port`]. An output port.
pub type SendPort<H> = Port<SEND, H>;
/// Recv-specific variant of [`Port`]. An input port.
pub type RecvPort<H> = Port<RECV, H>;

/// Methods for [`TeeingHandoff`] teeing and dropping.
impl<T: Clone> RecvPort<TeeingHandoff<T>> {
    /// Tees this [`TeeingHandoff`], given the [`Dfir`] instance it belongs to.
    pub fn tee(&self, hf: &mut Dfir) -> RecvPort<TeeingHandoff<T>> {
        hf.teeing_handoff_tee(self)
    }

    /// Marks this output of a [`TeeingHandoff`] as dropped so that no more data will be sent to
    /// it, given the [`Dfir`] instance it belongs to.
    ///
    /// It is recommended to not not use this method and instead simply avoid teeing a
    /// [`TeeingHandoff`] when it is not needed.
    pub fn drop(self, hf: &mut Dfir) {
        hf.teeing_handoff_drop(self)
    }
}

/// Wrapper around a handoff to differentiate between output and input.
#[derive(RefCast)]
#[repr(transparent)]
pub struct PortCtx<S: Polarity, H> {
    pub(crate) handoff: H,
    pub(crate) _marker: PhantomData<*const S>,
}
/// Send-specific [`PortCtx`]. Output to send into a handoff.
pub type SendCtx<H> = PortCtx<SEND, H>;
/// Recv-specific [`PortCtx`]. Input to receive from a handoff.
pub type RecvCtx<H> = PortCtx<RECV, H>;

/// Context provided to a subgraph for reading from a handoff. Corresponds to a [`SendPort`].
impl<H: Handoff> SendCtx<H> {
    /// Alias for [`Handoff::give`] on the inner `H`.
    pub fn give<T>(&self, item: T) -> T
    where
        H: CanReceive<T>,
    {
        <H as CanReceive<T>>::give(&self.handoff, item)
    }

    /// Alias for [`Handoff::try_give`] on the inner `H`.
    pub fn try_give<T>(&self, item: T) -> Result<T, T>
    where
        H: TryCanReceive<T>,
    {
        <H as TryCanReceive<T>>::try_give(&self.handoff, item)
    }
}

/// Context provided to a subgraph for reading from a handoff. Corresponds to a [`RecvPort`].
impl<H: Handoff> RecvCtx<H> {
    /// See [`Handoff::take_inner`].
    pub fn take_inner(&self) -> H::Inner {
        self.handoff.take_inner()
    }

    /// See [`Handoff::borrow_mut_swap`].
    pub fn borrow_mut_swap(&self) -> RefMut<'_, H::Inner> {
        self.handoff.borrow_mut_swap()
    }
}
