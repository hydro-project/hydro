//! Organizational module for Hydroflow Send/RecvCtx structs and Input/OutputPort structs.
use std::cell::RefCell;
use std::rc::Rc;

use crate::scheduled::handoff::{CanReceive, Handoff, TeeingHandoff, TryCanReceive};
use crate::scheduled::util::{Once, SendOnce};
use crate::scheduled::OpId;

/**
 * Context provided to a compiled component for writing to an [OutputPort].
 */
pub struct SendCtx<H: Handoff> {
    pub(crate) handoff: Rc<RefCell<H>>,
}
impl<H: Handoff> SendCtx<H> {
    // // TODO: represent backpressure in this return value.
    // #[allow(clippy::result_unit_err)]
    // pub fn give(self, item: H::Item) -> Result<(), ()> {
    //     (*self.once.get()).borrow_mut().try_give(item)
    // }
    pub fn give<T>(&self, item: T) -> T
    where
        H: CanReceive<T>,
    {
        <H as CanReceive<T>>::give(&self.handoff.borrow(), item)
    }

    pub fn try_give<T>(&self, item: T) -> Result<T, T>
    where
        H: TryCanReceive<T>,
    {
        <H as TryCanReceive<T>>::try_give(&self.handoff.borrow(), item)
    }
}

/**
 * Handle corresponding to a [SendCtx]. Consumed by [crate::scheduled::Hydroflow::add_edge] to construct the Hydroflow graph.
 */
#[must_use]
pub struct OutputPort<H: Handoff> {
    pub(crate) op_id: OpId,
    pub(crate) handoff: Rc<RefCell<H>>,
}
impl<H: Handoff> OutputPort<H> {
    pub fn op_id(&self) -> OpId {
        self.op_id
    }
}
impl<T: Clone> Clone for OutputPort<TeeingHandoff<T>> {
    fn clone(&self) -> Self {
        Self {
            op_id: self.op_id,
            handoff: Rc::new(RefCell::new(self.handoff.borrow().tee())),
        }
    }
}

/**
 * Context provided to a compiled component for reading from an [InputPort].
 */
pub struct RecvCtx<H: Handoff> {
    pub(crate) once: Once<Rc<RefCell<H>>>,
}
impl<H: Handoff> RecvCtx<H> {
    pub fn take_inner(&self) -> H::Inner {
        (*self.once.get().borrow_mut()).take_inner()
    }
}

/**
 * Handle corresponding to a [RecvCtx]. Consumed by [crate::scheduled::Hydroflow::add_edge] to construct the Hydroflow graph.
 */
// TODO: figure out how to explain succinctly why this and output port both use Writable
#[must_use]
pub struct InputPort<H: Handoff> {
    pub(crate) op_id: OpId,
    pub(crate) once: SendOnce<Rc<RefCell<H>>>,
}
impl<H: Handoff> InputPort<H> {
    pub fn op_id(&self) -> OpId {
        self.op_id
    }
}
