//! Shared test utilities for Push tests.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::pin::Pin;

use crate::push::{Push, PushStep};
use crate::{No, Toggle};

/// Records which method was called on a [`TestPush`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushCall<Item> {
    /// `poll_ready` was called.
    PollReady,
    /// `start_send` was called with the given item.
    SendItem(Item),
    /// `poll_flush` was called.
    PollFlush,
}

/// A configurable test push that replays separate logs of [`PushStep`]s for
/// `poll_ready` and `poll_flush`, records a history of all calls, and enforces
/// Push protocol invariants.
///
/// # Generic Parameters
///
/// - `Item`: The type of items accepted by this push.
/// - `CanPend`: A [`Toggle`] type (`Yes` or `No`) that statically encodes
///   whether this push can return [`PushStep::Pending`]. When set to [`No`],
///   the `Pending` variant cannot be constructed.
/// - `FUSED`: When `true`, exhausted step logs return [`PushStep::Done`]
///   instead of panicking. When `false`, polling after the log is exhausted
///   will panic.
///
/// # Panics
///
/// - `start_send` is called without a preceding `poll_ready` returning `Done`.
/// - When `FUSED` is `false`, `poll_ready` or `poll_flush` is called after
///   the corresponding step log is exhausted.
pub struct TestPush<Item, CanPend: Toggle, const FUSED: bool> {
    /// Steps returned by `poll_ready`, consumed in order.
    ready_steps: VecDeque<PushStep<CanPend>>,
    /// Steps returned by `poll_flush`, consumed in order.
    flush_steps: VecDeque<PushStep<CanPend>>,
    /// Recorded history of calls.
    pub history: Vec<PushCall<Item>>,
    ready: bool,
}

impl<Item, CanPend: Toggle, const FUSED: bool> TestPush<Item, CanPend, FUSED> {
    /// Creates a new `TestPush` from separate step logs for `poll_ready` and `poll_flush`.
    fn new(
        ready_steps: impl IntoIterator<Item = PushStep<CanPend>>,
        flush_steps: impl IntoIterator<Item = PushStep<CanPend>>,
    ) -> Self {
        Self {
            ready_steps: ready_steps.into_iter().collect(),
            flush_steps: flush_steps.into_iter().collect(),
            history: Vec::new(),
            ready: false,
        }
    }

    /// Returns the items sent via `start_send`, extracted from the call history.
    pub fn items(&self) -> Vec<Item>
    where
        Item: Clone,
    {
        self.history
            .iter()
            .filter_map(|c| match c {
                PushCall::SendItem(x) => Some(x.clone()),
                _ => None,
            })
            .collect()
    }
}

impl<Item, CanPend: Toggle> TestPush<Item, CanPend, true> {
    /// Creates a new `TestPush` from separate step logs for `poll_ready` and `poll_flush`.
    pub(crate) fn new_fused(
        ready_steps: impl IntoIterator<Item = PushStep<CanPend>>,
        flush_steps: impl IntoIterator<Item = PushStep<CanPend>>,
    ) -> Self {
        Self::new(ready_steps, flush_steps)
    }
}

impl<Item> TestPush<Item, No, true> {
    /// Creates a `TestPush` with `CanPend = No` and empty step logs.
    ///
    /// Always returns `Done` for `poll_ready` and `poll_flush`.
    pub(crate) fn no_pend() -> Self {
        Self::new([], [])
    }
}

impl<Item, CanPend: Toggle, const FUSED: bool> Unpin for TestPush<Item, CanPend, FUSED> {}

impl<Item, CanPend: Toggle, const FUSED: bool> Push<Item, ()> for TestPush<Item, CanPend, FUSED> {
    type Ctx<'ctx> = ();
    type CanPend = CanPend;

    fn poll_ready(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<CanPend> {
        let this = self.get_mut();
        this.history.push(PushCall::PollReady);
        let step = match this.ready_steps.pop_front() {
            Some(step) => step,
            None if FUSED => PushStep::Done,
            None => panic!("TestPush: poll_ready after log exhausted",),
        };
        this.ready = step.is_done();
        step
    }

    fn start_send(self: Pin<&mut Self>, item: Item, _meta: ()) {
        let this = self.get_mut();
        assert!(
            this.ready,
            "TestPush: start_send called without poll_ready returning Done"
        );
        this.ready = false;
        this.history.push(PushCall::SendItem(item));
    }

    fn poll_flush(self: Pin<&mut Self>, _ctx: &mut ()) -> PushStep<CanPend> {
        let this = self.get_mut();
        this.history.push(PushCall::PollFlush);
        match this.flush_steps.pop_front() {
            Some(step) => step,
            None if FUSED => PushStep::Done,
            None => panic!("TestPush: poll_flush after log exhausted"),
        }
    }
}

/// Compile-time assertion that a push has `CanPend = No`.
pub fn assert_can_pend_no<T, M: Copy>(_push: &impl Push<T, M, CanPend = No>) {}
