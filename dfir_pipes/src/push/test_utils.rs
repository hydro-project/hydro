//! Shared test utilities for Push tests.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::pin::Pin;

use crate::No;
use crate::push::{Push, PushStep};

/// A simple push that collects items into a shared `Vec`.
///
/// Useful for testing push pipelines by inspecting collected output.
#[derive(Default)]
pub struct CollectPush<T> {
    /// Shared storage for collected items.
    pub items: Rc<RefCell<Vec<T>>>,
}

impl<T> Push<T, ()> for CollectPush<T> {
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<No> {
        PushStep::Done
    }
    fn start_send(self: Pin<&mut Self>, item: T, _meta: ()) {
        self.get_mut().items.borrow_mut().push(item);
    }
    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<No> {
        PushStep::Done
    }
}

/// Mock push for testing async push operators.
#[derive(Default)]
pub struct AsyncMockPush<T> {
    pub items: Vec<T>,
    pub poll_ready_count: usize,
    pub poll_flush_count: usize,
}

impl<T> Unpin for AsyncMockPush<T> {}

impl<T> Push<T, ()> for AsyncMockPush<T> {
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<No> {
        self.get_mut().poll_ready_count += 1;
        PushStep::Done
    }

    fn start_send(self: Pin<&mut Self>, item: T, _meta: ()) {
        self.get_mut().items.push(item);
    }

    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut Self::Ctx<'_>) -> PushStep<No> {
        self.get_mut().poll_flush_count += 1;
        PushStep::Done
    }
}
