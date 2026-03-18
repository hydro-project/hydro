use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};
use crate::{Context, Toggle, Yes};

pin_project! {
    /// Pull combinator that chains two pulls in sequence.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Chain<A, B> {
        #[pin]
        first: A,
        #[pin]
        second: B,
    }
}

impl<A, B> Chain<A, B>
where
    Self: Pull,
{
    pub(crate) const fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<A, B> Pull for Chain<A, B>
where
    A: FusedPull<CanEnd = Yes>,
    B: Pull<Item = A::Item, Meta = A::Meta>,
{
    type Ctx<'ctx> = <A::Ctx<'ctx> as Context<'ctx>>::Merged<B::Ctx<'ctx>>;

    type Item = A::Item;
    type Meta = A::Meta;
    type CanPend = <A::CanPend as Toggle>::Or<B::CanPend>;
    type CanEnd = B::CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        match this.first.as_mut().pull(Context::unmerge_self(ctx)) {
            PullStep::Ready(item, meta) => {
                return PullStep::Ready(item, meta);
            }
            PullStep::Pending(_) => {
                return PullStep::pending();
            }
            PullStep::Ended(_) => {
                // First is fused, so it will keep returning Ended.
                // Fall through to pull from second.
            }
        }

        this.second
            .as_mut()
            .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
            .convert_into()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.first.size_hint();
        let (b_lower, b_upper) = self.second.size_hint();

        let lower = a_lower.saturating_add(b_lower);
        let upper = a_upper.zip(b_upper).and_then(|(a, b)| a.checked_add(b));

        (lower, upper)
    }
}

impl<A, B> FusedPull for Chain<A, B>
where
    A: FusedPull<CanEnd = Yes>,
    B: FusedPull<Item = A::Item, Meta = A::Meta>,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pull::test_utils::{TestPull, assert_is_fused, assert_types};
    use crate::pull::{No, Repeat, Yes, pending};

    /// Fused pull with CanPend=No, CanEnd=Yes.
    type SyncTestPull = TestPull<i32, (), No, Yes, true>;
    /// Fused pull with CanPend=Yes, CanEnd=Yes.
    type AsyncTestPull = TestPull<i32, (), Yes, Yes, true>;

    #[test]
    fn chain_sync_with_various_second() {
        // Sync + Sync: CanPend=No, CanEnd=Yes
        let chain = Chain::new(SyncTestPull::new([]), SyncTestPull::new([]));
        assert_types::<No, Yes>(&chain);
        assert_is_fused(&chain);

        // Sync + Infinite: CanPend=No, CanEnd=No
        let chain = Chain::new(SyncTestPull::new([]), Repeat::new(42));
        assert_types::<No, No>(&chain);

        // Sync + Pending: CanPend=Yes (No.or(Yes)), CanEnd=No
        let chain = Chain::new(SyncTestPull::new([]), pending());
        assert_types::<Yes, No>(&chain);
    }

    #[test]
    fn chain_async_with_various_second() {
        // Async + Async: CanPend=Yes, CanEnd=Yes
        let chain = Chain::new(AsyncTestPull::new([]), AsyncTestPull::new([]));
        assert_types::<Yes, Yes>(&chain);
        assert_is_fused(&chain);

        // Async + Infinite: CanPend=Yes, CanEnd=No
        let chain = Chain::new(AsyncTestPull::new([]), Repeat::new(42));
        assert_types::<Yes, No>(&chain);
    }

    #[test]
    fn chain_nested_types() {
        // Chain<Chain<Sync, Async>, Infinite>: CanPend=Yes, CanEnd=No
        let chain_ab = Chain::new(SyncTestPull::new([]), AsyncTestPull::new([]));
        assert_types::<Yes, Yes>(&chain_ab);

        let chain_abc = Chain::new(chain_ab, Repeat::new(3));
        assert_types::<Yes, No>(&chain_abc);
    }

    #[test]
    fn chain_fused_shields_upstream() {
        use core::pin::pin;

        use crate::pull::once;
        use crate::pull::test_utils::assert_fused_runtime;

        // TODO(mingwei): use upstream `Pull`s that pend sometimes.
        let p = pin!(once(5).chain(once(6)));
        assert_fused_runtime(p);
    }
}
