//! Shared test utilities for Pull type algebra tests.

use alloc::collections::VecDeque;
use core::pin::Pin;

use crate::pull::{FusedPull, Pull, PullStep};
use crate::{No, Toggle, Yes};

/// A configurable test pull that replays a log of [`PullStep`]s in order.
///
/// Each call to [`Pull::pull`] pops the next step from the front of the log.
///
/// When the log is empty:
/// - For non-fused pulls (`FUSED = false`), this panics (to test fusing — the last
///   step should typically be `Ended`).
/// - For fused pulls (`FUSED = true`), this returns [`PullStep::ended`].
///
/// Generic over `CanPend`, `CanEnd` (for type algebra tests), and `FUSED`
/// (when `true`, implements [`FusedPull`]).
pub struct TestPull<Item, Meta: Copy, CanPend: Toggle, CanEnd: Toggle, const FUSED: bool> {
    steps: VecDeque<PullStep<Item, Meta, CanPend, CanEnd>>,
}

impl<Item, Meta: Copy, CanPend: Toggle, CanEnd: Toggle, const FUSED: bool>
    TestPull<Item, Meta, CanPend, CanEnd, FUSED>
{
    /// Creates a new `TestPull` from the given sequence of steps.
    pub(crate) fn new(
        steps: impl IntoIterator<Item = PullStep<Item, Meta, CanPend, CanEnd>>,
    ) -> Self {
        Self {
            steps: steps.into_iter().collect(),
        }
    }
}

impl<Item> TestPull<Item, (), No, Yes, false> {
    /// Creates a non-fused `TestPull` that yields each item as `Ready`, then `Ended`.
    /// Panics if polled again after the log is exhausted.
    pub(crate) fn items(items: impl IntoIterator<Item = Item>) -> Self {
        Self::new(
            items
                .into_iter()
                .map(|item| PullStep::Ready(item, ()))
                .chain(core::iter::once(PullStep::Ended(Yes))),
        )
    }
}

impl<Item> TestPull<Item, (), No, Yes, true> {
    /// Creates a fused `TestPull` that yields each item as `Ready`, then `Ended`.
    /// After the log is exhausted, further polls return [`PullStep::ended`].
    pub(crate) fn items_fused(items: impl IntoIterator<Item = Item>) -> Self {
        Self::new(
            items
                .into_iter()
                .map(|item| PullStep::Ready(item, ()))
                .chain(core::iter::once(PullStep::Ended(Yes))),
        )
    }
}

impl<Item, Meta: Copy, CanPend: Toggle, CanEnd: Toggle, const FUSED: bool> Unpin
    for TestPull<Item, Meta, CanPend, CanEnd, FUSED>
{
}

impl<Item, Meta: Copy, CanPend: Toggle, CanEnd: Toggle, const FUSED: bool> Pull
    for TestPull<Item, Meta, CanPend, CanEnd, FUSED>
{
    type Ctx<'ctx> = ();
    type Item = Item;
    type Meta = Meta;
    type CanPend = CanPend;
    type CanEnd = CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        _ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        match self.get_mut().steps.pop_front() {
            Some(step) => step,
            None if FUSED => PullStep::ended(),
            None => panic!("TestPull: polled after log exhausted"),
        }
    }
}

impl<Item, Meta: Copy, CanPend: Toggle, CanEnd: Toggle> FusedPull
    for TestPull<Item, Meta, CanPend, CanEnd, true>
{
}

/// Compile-time assertion that a pull implements [`FusedPull`].
pub fn assert_is_fused(_: &impl FusedPull) {}

/// Drains a fused pull to Ended, then polls once more to verify it returns Ended
/// (and does not poll the upstream again, which would panic via [`TestPull`]).
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
