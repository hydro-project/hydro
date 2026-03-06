use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Context, FusedPull, Pull, Step, Toggle};

pin_project! {
    /// Asynchronously merges two upstream pulls, interleaving their items.
    ///
    /// Unlike [`Chain`](super::chain::Chain), `Merge` does not require the first
    /// pull to be finite. Items are pulled from both sources in a round-robin
    /// fashion, and the merged pull only ends when both upstream pulls have ended.
    ///
    /// Both upstream pulls must be fused ([`FusedPull`]) to ensure correct
    /// behavior after one pull ends.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Merge<A, B> {
        #[pin]
        first: A,
        #[pin]
        second: B,
        poll_first_next: bool,
    }
}

impl<A, B> Merge<A, B>
where
    Self: Pull,
{
    pub(crate) const fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            poll_first_next: true,
        }
    }
}

impl<A, B> Pull for Merge<A, B>
where
    A: FusedPull,
    B: FusedPull<Item = A::Item, Meta = A::Meta>,
{
    type Ctx<'ctx> = <A::Ctx<'ctx> as Context<'ctx>>::Merged<B::Ctx<'ctx>>;

    type Item = A::Item;
    type Meta = A::Meta;
    type CanPend = <A::CanPend as Toggle>::Or<B::CanPend>;
    type CanEnd = <A::CanEnd as Toggle>::And<B::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        let (first_result, second_result) = if *this.poll_first_next {
            *this.poll_first_next = false;
            let first = this
                .first
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_self(ctx));
            match first {
                Step::Ready(item, meta) => return Step::Ready(item, meta),
                Step::Pending(_) => return Step::pending(),
                Step::Ended(_) => {}
            }
            let second = this
                .second
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx));
            (None, Some(second))
        } else {
            *this.poll_first_next = true;
            let second = this
                .second
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx));
            match second {
                Step::Ready(item, meta) => return Step::Ready(item, meta),
                Step::Pending(_) => return Step::pending(),
                Step::Ended(_) => {}
            }
            let first = this
                .first
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_self(ctx));
            (Some(first), None)
        };

        // The preferred side ended, try the other side.
        if let Some(second) = second_result {
            match second {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(_) => Step::pending(),
                Step::Ended(_) => Step::ended(),
            }
        } else if let Some(first) = first_result {
            match first {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(_) => Step::pending(),
                Step::Ended(_) => Step::ended(),
            }
        } else {
            unreachable!()
        }
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

impl<A, B> FusedPull for Merge<A, B>
where
    A: FusedPull,
    B: FusedPull<Item = A::Item, Meta = A::Meta>,
{
}

#[cfg(test)]
mod tests {
    use super::Merge;
    use crate::test_utils::{AsyncPull, SyncPull, assert_types};
    use crate::{No, Pending, Repeat, Yes};

    // Merge allows both pulls to be infinite (unlike Chain).
    // CanPend = A::CanPend.or(B::CanPend), CanEnd = A::CanEnd.and(B::CanEnd)

    #[test]
    fn merge_finite_pulls() {
        // Sync + Sync: CanPend=No, CanEnd=Yes
        let merge: Merge<SyncPull, SyncPull> = Merge::new(SyncPull::new(1), SyncPull::new(1));
        assert_types::<No, Yes>(&merge);

        // Async + Async: CanPend=Yes, CanEnd=Yes
        let merge: Merge<AsyncPull, AsyncPull> = Merge::new(AsyncPull::new(1), AsyncPull::new(1));
        assert_types::<Yes, Yes>(&merge);
    }

    #[test]
    fn merge_with_infinite_pulls() {
        // Sync + Infinite: CanPend=No, CanEnd=No (Yes.and(No))
        let merge: Merge<SyncPull, Repeat<i32>> = Merge::new(SyncPull::new(1), Repeat::new(42));
        assert_types::<No, No>(&merge);

        // Infinite + Infinite: CanPend=No, CanEnd=No - key difference from Chain!
        let merge: Merge<Repeat<i32>, Repeat<i32>> = Merge::new(Repeat::new(1), Repeat::new(2));
        assert_types::<No, No>(&merge);

        // Pending + Infinite: CanPend=Yes, CanEnd=No
        let merge: Merge<Pending<i32>, Repeat<i32>> = Merge::new(crate::pending(), Repeat::new(42));
        assert_types::<Yes, No>(&merge);
    }

    #[test]
    fn merge_nested_types() {
        // Merge<Merge<Sync, Async>, Infinite>: CanPend=Yes, CanEnd=No
        let merge_ab: Merge<SyncPull, AsyncPull> = Merge::new(SyncPull::new(1), AsyncPull::new(1));
        assert_types::<Yes, Yes>(&merge_ab);

        let merge_abc: Merge<Merge<SyncPull, AsyncPull>, Repeat<i32>> =
            Merge::new(merge_ab, Repeat::new(3));
        assert_types::<Yes, No>(&merge_abc);
    }
}
