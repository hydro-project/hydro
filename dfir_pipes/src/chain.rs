use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Context, FusedPull, Pull, Step, Toggle, Yes};

pin_project! {
    pub struct Chain<A, B> {
        #[pin]
        first: A,
        #[pin]
        second: B,
    }
}

impl<A, B> Chain<A, B> {
    pub fn new(first: A, second: B) -> Self {
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        match this.first.as_mut().pull(Context::unmerge_self(ctx)) {
            Step::Ready(item, meta) => {
                return Step::Ready(item, meta);
            }
            Step::Pending(can_pend) => {
                return Step::Pending(Toggle::convert_from(can_pend));
            }
            Step::Ended(_) => {
                // First is fused, so it will keep returning Ended.
                // Fall through to pull from second.
            }
        }

        this.second
            .as_mut()
            .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
            .convert_into()
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let (a_lower, a_upper) = this.first.size_hint();
        let (b_lower, b_upper) = this.second.size_hint();

        let lower = a_lower.saturating_add(b_lower);
        let upper = match (a_upper, b_upper) {
            (Some(a), Some(b)) => a.checked_add(b),
            _ => None,
        };

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
    use super::Chain;
    use crate::test_utils::{AsyncPull, InfinitePull, PendingPull, SyncPull, assert_types};
    use crate::{No, Yes};

    // Chain requires A::CanEnd = Yes, so first pull must be finite.
    // CanPend = A::CanPend.or(B::CanPend), CanEnd = B::CanEnd

    #[test]
    fn chain_sync_with_various_second() {
        // Sync + Sync: CanPend=No, CanEnd=Yes
        let chain: Chain<SyncPull, SyncPull> = Chain::new(SyncPull::new(1), SyncPull::new(1));
        assert_types::<No, Yes>(&chain);

        // Sync + Infinite: CanPend=No, CanEnd=No
        let chain: Chain<SyncPull, InfinitePull> =
            Chain::new(SyncPull::new(1), InfinitePull::new(42));
        assert_types::<No, No>(&chain);

        // Sync + Pending: CanPend=Yes (No.or(Yes)), CanEnd=No
        let chain: Chain<SyncPull, PendingPull<i32>> =
            Chain::new(SyncPull::new(1), PendingPull::new());
        assert_types::<Yes, No>(&chain);
    }

    #[test]
    fn chain_async_with_various_second() {
        // Async + Async: CanPend=Yes, CanEnd=Yes
        let chain: Chain<AsyncPull, AsyncPull> = Chain::new(AsyncPull::new(1), AsyncPull::new(1));
        assert_types::<Yes, Yes>(&chain);

        // Async + Infinite: CanPend=Yes, CanEnd=No
        let chain: Chain<AsyncPull, InfinitePull> =
            Chain::new(AsyncPull::new(1), InfinitePull::new(42));
        assert_types::<Yes, No>(&chain);
    }

    #[test]
    fn chain_nested_types() {
        // Chain<Chain<Sync, Async>, Infinite>: CanPend=Yes, CanEnd=No
        let chain_ab: Chain<SyncPull, AsyncPull> = Chain::new(SyncPull::new(1), AsyncPull::new(1));
        assert_types::<Yes, Yes>(&chain_ab);

        let chain_abc: Chain<Chain<SyncPull, AsyncPull>, InfinitePull> =
            Chain::new(chain_ab, InfinitePull::new(3));
        assert_types::<Yes, No>(&chain_abc);
    }
}
