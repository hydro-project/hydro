//! Shared test utilities for Pull type algebra tests.

use core::pin::Pin;

use crate::pull::{FusedPull, Pull, PullStep};
use crate::{No, Toggle, Yes};

/// Helper pull that can pend and can end (CanPend=Yes, CanEnd=Yes).
/// This pull is fused - once ended, it stays ended.
pub struct AsyncPull {
    count: usize,
    max: usize,
    pending_next: bool,
    ended: bool,
}

impl AsyncPull {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            count: 0,
            max,
            pending_next: false,
            ended: false,
        }
    }
}

impl Pull for AsyncPull {
    type Ctx<'ctx> = ();

    type Item = i32;
    type Meta = ();
    type CanPend = Yes;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        if this.ended {
            return PullStep::Ended(Yes);
        }
        if this.pending_next {
            this.pending_next = false;
            PullStep::Pending(Yes)
        } else if this.count < this.max {
            let item = this.count as i32;
            this.count += 1;
            this.pending_next = true;
            PullStep::Ready(item, ())
        } else {
            this.ended = true;
            PullStep::Ended(Yes)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.ended {
            (0, Some(0))
        } else {
            let remaining = self.max.saturating_sub(self.count);
            (remaining, Some(remaining))
        }
    }
}

impl FusedPull for AsyncPull {}

/// Helper pull that never pends but can end (CanPend=No, CanEnd=Yes).
/// This pull is fused - once ended, it stays ended.
pub struct SyncPull {
    count: usize,
    max: usize,
    ended: bool,
}

impl SyncPull {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            count: 0,
            max,
            ended: false,
        }
    }
}

impl Pull for SyncPull {
    type Ctx<'ctx> = ();

    type Item = i32;
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        if this.ended {
            return PullStep::Ended(Yes);
        }
        if this.count < this.max {
            let item = this.count as i32;
            this.count += 1;
            PullStep::Ready(item, ())
        } else {
            this.ended = true;
            PullStep::Ended(Yes)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.ended {
            (0, Some(0))
        } else {
            let remaining = self.max.saturating_sub(self.count);
            (remaining, Some(remaining))
        }
    }
}

impl FusedPull for SyncPull {}

/// Compile-time assertion that a pull implements [`FusedPull`].
pub fn assert_is_fused(_: &impl FusedPull) {}

/// Helper pull that panics if polled after returning Ended.
/// Use this as upstream for fused combinator tests — if the combinator
/// correctly shields the upstream after ending, the panic is never hit.
pub struct PanicsAfterEndPull {
    count: usize,
    max: usize,
    ended: bool,
}

impl PanicsAfterEndPull {
    pub(crate) fn new(max: usize) -> Self {
        Self {
            count: 0,
            max,
            ended: false,
        }
    }
}

impl Pull for PanicsAfterEndPull {
    type Ctx<'ctx> = ();
    type Item = i32;
    type Meta = ();
    type CanPend = No;
    type CanEnd = Yes;

    fn pull(self: Pin<&mut Self>, _ctx: &mut ()) -> PullStep<i32, (), No, Yes> {
        let this = self.get_mut();
        assert!(!this.ended, "PanicsAfterEndPull: polled after Ended");
        if this.count < this.max {
            let item = this.count as i32;
            this.count += 1;
            PullStep::Ready(item, ())
        } else {
            this.ended = true;
            PullStep::Ended(Yes)
        }
    }
}

impl FusedPull for PanicsAfterEndPull {}

/// Drains a fused pull to Ended, then polls once more to verify it returns Ended
/// (and does not poll the upstream again, which would panic via [`PanicsAfterEndPull`]).
pub fn assert_fused_runtime<P>(mut pull: Pin<&mut P>)
where
    P: for<'ctx> FusedPull<CanEnd = Yes, Ctx<'ctx> = ()>,
{
    loop {
        match pull.as_mut().pull(&mut ()) {
            PullStep::Ready(_, _) => {}
            PullStep::Pending(_) => {}
            PullStep::Ended(_) => break,
        }
    }
    for _ in 0..5 {
        assert!(
            pull.as_mut().pull(&mut ()).is_ended(),
            "FusedPull returned non-Ended after Ended"
        );
    }
}

/// Compile-time assertion helper for type equality.
pub fn assert_types<CanPend: Toggle, CanEnd: Toggle>(
    _: &impl Pull<CanPend = CanPend, CanEnd = CanEnd>,
) {
}

/// A non-fused pull that yields items [0..max), then Ended, then re-yields items if polled again.
pub struct NonFusedPull {
    pub count: usize,
    pub max: usize,
}

impl NonFusedPull {
    pub(crate) fn new(max: usize) -> Self {
        Self { count: 0, max }
    }
}

impl Pull for NonFusedPull {
    type Ctx<'ctx> = ();
    type Item = i32;
    type Meta = ();
    type CanPend = Yes;
    type CanEnd = Yes;

    fn pull(self: Pin<&mut Self>, _ctx: &mut ()) -> PullStep<i32, (), Yes, Yes> {
        let this = self.get_mut();
        if this.count < this.max {
            let item = this.count as i32;
            this.count += 1;
            PullStep::Ready(item, ())
        } else {
            this.count = 0;
            PullStep::Ended(Yes)
        }
    }
}

/// A `futures_sink::Sink` that collects i32 items and returns Pending from poll_flush a configurable number of times.
pub struct PendingFlushSink {
    pub items: alloc::vec::Vec<i32>,
    pub flush_pending_count: usize,
}

impl futures_sink::Sink<i32> for PendingFlushSink {
    type Error = core::convert::Infallible;

    fn poll_ready(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Result<(), Self::Error>> {
        core::task::Poll::Ready(Ok(()))
    }
    fn start_send(self: Pin<&mut Self>, item: i32) -> Result<(), Self::Error> {
        self.get_mut().items.push(item);
        Ok(())
    }
    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        if this.flush_pending_count > 0 {
            this.flush_pending_count -= 1;
            core::task::Poll::Pending
        } else {
            core::task::Poll::Ready(Ok(()))
        }
    }
    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Result<(), Self::Error>> {
        core::task::Poll::Ready(Ok(()))
    }
}
