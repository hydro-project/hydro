//! Shared test utilities for Push tests.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::pin::Pin;

use crate::No;
use crate::push::{Push, PushStep};

/// Compile-time assertion that a push has `CanPend = No`.
pub fn assert_can_pend_no<T, M: Copy>(_push: &impl Push<T, M, CanPend = No>) {}

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

/// A push that collects i32 items and returns Pending from poll_flush a configurable number of times.
pub struct PendingFlushPush {
    pub items: Vec<i32>,
    pub flush_pending_count: usize,
}

impl Push<i32, ()> for PendingFlushPush {
    type Ctx<'ctx> = ();
    type CanPend = crate::Yes;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<crate::Yes> {
        PushStep::Done
    }
    fn start_send(self: Pin<&mut Self>, item: i32, _meta: ()) {
        self.get_mut().items.push(item);
    }
    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<crate::Yes> {
        let this = self.get_mut();
        if this.flush_pending_count > 0 {
            this.flush_pending_count -= 1;
            PushStep::Pending(crate::Yes)
        } else {
            PushStep::Done
        }
    }
}

/// A push that panics if `start_send` is called without a preceding `poll_ready` returning `Done`.
/// Collects items for inspection.
pub struct ReadyGuardPush<T> {
    pub items: Vec<T>,
    ready: bool,
}

impl<T> ReadyGuardPush<T> {
    pub(crate) fn new() -> Self {
        Self {
            items: Vec::new(),
            ready: false,
        }
    }
}

impl<T: Unpin> Push<T, ()> for ReadyGuardPush<T> {
    type Ctx<'ctx> = ();
    type CanPend = No;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<No> {
        self.get_mut().ready = true;
        PushStep::Done
    }
    fn start_send(self: Pin<&mut Self>, item: T, _meta: ()) {
        let this = self.get_mut();
        assert!(
            this.ready,
            "ReadyGuardPush: start_send called without poll_ready"
        );
        this.ready = false;
        this.items.push(item);
    }
    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<No> {
        PushStep::Done
    }
}
