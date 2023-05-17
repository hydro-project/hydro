//! Organizational module for Hydroflow Send/RecvCtx structs and Input/OutputPort structs.
use std::cell::RefMut;
use std::marker::PhantomData;

use ref_cast::RefCast;
use sealed::sealed;

use crate::scheduled::handoff::{CanReceive, Handoff, TryCanReceive};

use super::HandoffId;

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
pub enum SEND {}
/// An uninstantiable type used to tag port [`Polarity`] as **receive**.
///
/// See also: [`SEND`].
pub enum RECV {}
#[sealed]
impl Polarity for SEND {}
#[sealed]
impl Polarity for RECV {}

#[must_use]
pub struct Port<S: Polarity, H>
where
    H: Handoff,
{
    pub(crate) handoff_id: HandoffId,
    #[allow(clippy::type_complexity)]
    pub(crate) _marker: PhantomData<(*const S, fn() -> H)>,
}
pub type SendPort<H> = Port<SEND, H>;
pub type RecvPort<H> = Port<RECV, H>;

#[derive(RefCast)]
#[repr(transparent)]
pub struct PortCtx<S: Polarity, H> {
    pub(crate) handoff: H,
    pub(crate) _marker: PhantomData<*const S>,
}

pub type SendCtx<H> = PortCtx<SEND, H>;
pub type RecvCtx<H> = PortCtx<RECV, H>;

/// Context provided to a subgraph for reading from a handoff. Corresponds to a [`SendPort`].
impl<H: Handoff> SendCtx<H> {
    // TODO: represent backpressure in this return value.
    pub fn give<T>(&self, item: T) -> T
    where
        H: CanReceive<T>,
    {
        <H as CanReceive<T>>::give(&self.handoff, item)
    }

    pub fn try_give<T>(&self, item: T) -> Result<T, T>
    where
        H: TryCanReceive<T>,
    {
        <H as TryCanReceive<T>>::try_give(&self.handoff, item)
    }
}

/// Context provided to a subgraph for reading from a handoff. Corresponds to a [`RecvPort`].
impl<H: Handoff> RecvCtx<H> {
    pub fn take_inner(&self) -> H::Inner {
        self.handoff.take_inner()
    }

    pub fn borrow_mut_swap(&self) -> RefMut<H::Inner> {
        self.handoff.borrow_mut_swap()
    }
}
