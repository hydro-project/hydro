use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Context, Pull, Step, Toggle, Yes};

pin_project! {
    /// Asynchronously merges two upstream pulls, interleaving their items.
    ///
    /// Unlike [`Chain`](super::chain::Chain), `Merge` does not require the first
    /// pull to be finite. Items are pulled from both sources in a round-robin
    /// fashion, and the merged pull only ends when both upstream pulls have ended.
    pub struct Merge<A, B> {
        #[pin]
        first: A,
        #[pin]
        second: B,
        first_ended: bool,
        second_ended: bool,
        poll_first_next: bool,
    }
}

impl<A, B> Merge<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self {
            first,
            second,
            first_ended: false,
            second_ended: false,
            poll_first_next: true,
        }
    }
}

impl<A, B> Pull for Merge<A, B>
where
    A: Pull,
    B: Pull<Item = A::Item, Meta = A::Meta>,
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

        // Both ended - return Ended
        if *this.first_ended && *this.second_ended {
            return Step::Ended(Toggle::convert_from(Yes));
        }

        // Only first ended - pull from second
        if *this.first_ended {
            return match this
                .second
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
            {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
                Step::Ended(can_end) => {
                    *this.second_ended = true;
                    Step::Ended(Toggle::convert_from(can_end))
                }
            };
        }

        // Only second ended - pull from first
        if *this.second_ended {
            return match this
                .first
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
            {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
                Step::Ended(can_end) => {
                    *this.first_ended = true;
                    Step::Ended(Toggle::convert_from(can_end))
                }
            };
        }

        // Both active - alternate between them
        if *this.poll_first_next {
            *this.poll_first_next = false;
            match this
                .first
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
            {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
                Step::Ended(_) => {
                    *this.first_ended = true;
                    // Try second immediately
                    match this
                        .second
                        .as_mut()
                        .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
                    {
                        Step::Ready(item, meta) => Step::Ready(item, meta),
                        Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
                        Step::Ended(can_end) => {
                            *this.second_ended = true;
                            Step::Ended(Toggle::convert_from(can_end))
                        }
                    }
                }
            }
        } else {
            *this.poll_first_next = true;
            match this
                .second
                .as_mut()
                .pull(<A::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
            {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
                Step::Ended(_) => {
                    *this.second_ended = true;
                    // Try first immediately
                    match this
                        .first
                        .as_mut()
                        .pull(<A::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
                    {
                        Step::Ready(item, meta) => Step::Ready(item, meta),
                        Step::Pending(can_pend) => Step::Pending(Toggle::convert_from(can_pend)),
                        Step::Ended(can_end) => {
                            *this.first_ended = true;
                            Step::Ended(Toggle::convert_from(can_end))
                        }
                    }
                }
            }
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();

        let (a_lower, a_upper) = if *this.first_ended {
            (0, Some(0))
        } else {
            this.first.size_hint()
        };

        let (b_lower, b_upper) = if *this.second_ended {
            (0, Some(0))
        } else {
            this.second.size_hint()
        };

        let lower = a_lower.saturating_add(b_lower);
        let upper = match (a_upper, b_upper) {
            (Some(a), Some(b)) => a.checked_add(b),
            _ => None,
        };

        (lower, upper)
    }
}

#[cfg(test)]
mod tests {
    use super::Merge;
    use crate::test_utils::{AsyncPull, InfinitePull, PendingPull, SyncPull, assert_types};
    use crate::{No, Yes};

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
        let merge: Merge<SyncPull, InfinitePull> =
            Merge::new(SyncPull::new(1), InfinitePull::new(42));
        assert_types::<No, No>(&merge);

        // Infinite + Infinite: CanPend=No, CanEnd=No - key difference from Chain!
        let merge: Merge<InfinitePull, InfinitePull> =
            Merge::new(InfinitePull::new(1), InfinitePull::new(2));
        assert_types::<No, No>(&merge);

        // Pending + Infinite: CanPend=Yes, CanEnd=No
        let merge: Merge<PendingPull<i32>, InfinitePull> =
            Merge::new(PendingPull::new(), InfinitePull::new(42));
        assert_types::<Yes, No>(&merge);
    }

    #[test]
    fn merge_nested_types() {
        // Merge<Merge<Sync, Async>, Infinite>: CanPend=Yes, CanEnd=No
        let merge_ab: Merge<SyncPull, AsyncPull> = Merge::new(SyncPull::new(1), AsyncPull::new(1));
        assert_types::<Yes, Yes>(&merge_ab);

        let merge_abc: Merge<Merge<SyncPull, AsyncPull>, InfinitePull> =
            Merge::new(merge_ab, InfinitePull::new(3));
        assert_types::<Yes, No>(&merge_abc);
    }
}
