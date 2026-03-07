//! Shared test utilities for Pull type algebra tests.

use core::pin::Pin;

use crate::{FusedPull, No, Pull, Step, Toggle, Yes};

/// Helper pull that can pend and can end (CanPend=Yes, CanEnd=Yes).
/// This pull is fused - once ended, it stays ended.
pub struct AsyncPull {
    count: usize,
    max: usize,
    pending_next: bool,
    ended: bool,
}

impl AsyncPull {
    pub fn new(max: usize) -> Self {
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        if this.ended {
            return Step::Ended(Yes);
        }
        if this.pending_next {
            this.pending_next = false;
            Step::Pending(Yes)
        } else if this.count < this.max {
            let item = this.count as i32;
            this.count += 1;
            this.pending_next = true;
            Step::Ready(item, ())
        } else {
            this.ended = true;
            Step::Ended(Yes)
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
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
    pub fn new(max: usize) -> Self {
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.get_mut();
        if this.ended {
            return Step::Ended(Yes);
        }
        if this.count < this.max {
            let item = this.count as i32;
            this.count += 1;
            Step::Ready(item, ())
        } else {
            this.ended = true;
            Step::Ended(Yes)
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        if self.ended {
            (0, Some(0))
        } else {
            let remaining = self.max.saturating_sub(self.count);
            (remaining, Some(remaining))
        }
    }
}

impl FusedPull for SyncPull {}

/// Compile-time assertion helper for type equality.
pub fn assert_types<CanPend: Toggle, CanEnd: Toggle>(
    _: &impl Pull<CanPend = CanPend, CanEnd = CanEnd>,
) {
}
