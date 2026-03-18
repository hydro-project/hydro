use core::pin::Pin;

use itertools::{Either, EitherOrBoth};
use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep, fuse_self};
use crate::{Context, Toggle};

pin_project! {
    /// A pull that zips two pulls together, continuing until both are exhausted.
    ///
    /// Unlike a regular zip which ends when either pull ends, `ZipLongest`
    /// continues until both pulls have ended, yielding [`EitherOrBoth`] values.
    ///
    /// Both upstream pulls must be fused ([`FusedPull`]) to ensure correct
    /// behavior after one pull ends.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct ZipLongest<Prev1, Prev2>
    where
        Prev1: Pull,
        Prev2: Pull,
    {
        #[pin]
        prev1: Prev1,
        #[pin]
        prev2: Prev2,
        // Store an item from whichever stream was ready first, while waiting for the other.
        // `Left` = item from prev1, `Right` = item from prev2.
        buffer: Option<Either<(Prev1::Item, Prev1::Meta), (Prev2::Item, Prev2::Meta)>>,
    }
}

impl<Prev1, Prev2> ZipLongest<Prev1, Prev2>
where
    Prev1: Pull,
    Prev2: Pull,
    Self: Pull,
{
    /// Create a new `ZipLongest` stream from two source streams.
    pub(crate) const fn new(prev1: Prev1, prev2: Prev2) -> Self {
        Self {
            prev1,
            prev2,
            buffer: None,
        }
    }
}

impl<Prev1, Prev2> Pull for ZipLongest<Prev1, Prev2>
where
    Prev1: FusedPull,
    Prev2: FusedPull<Meta = Prev1::Meta>,
{
    type Ctx<'ctx> = <Prev1::Ctx<'ctx> as Context<'ctx>>::Merged<Prev2::Ctx<'ctx>>;

    type Item = EitherOrBoth<Prev1::Item, Prev2::Item>;
    type Meta = Prev1::Meta;
    type CanPend = <Prev1::CanPend as Toggle>::Or<Prev2::CanPend>;
    type CanEnd = <Prev1::CanEnd as Toggle>::And<Prev2::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        let (pull_left, pull_right) = match this.buffer.take() {
            Some(Either::Left((left_item, left_meta))) => {
                (Some(PullStep::Ready(left_item, left_meta)), None)
            }
            Some(Either::Right((right_item, right_meta))) => {
                (None, Some(PullStep::Ready(right_item, right_meta)))
            }
            None => (None, None),
        };

        let pull_left = pull_left.unwrap_or_else(|| {
            this.prev1
                .as_mut()
                .pull(<Prev1::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
        });
        let pull_right = pull_right.unwrap_or_else(|| {
            this.prev2
                .as_mut()
                .pull(<Prev1::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
        });

        match (pull_left, pull_right) {
            (PullStep::Ready(left_item, left_meta), PullStep::Ready(right_item, _right_meta)) => {
                PullStep::Ready(EitherOrBoth::Both(left_item, right_item), left_meta)
            } // TODO(mingwei): use right_meta
            (PullStep::Ready(left_item, left_meta), PullStep::Ended(_)) => {
                PullStep::Ready(EitherOrBoth::Left(left_item), left_meta)
            }
            (PullStep::Ended(_), PullStep::Ready(right_item, right_meta)) => {
                PullStep::Ready(EitherOrBoth::Right(right_item), right_meta)
            }
            (PullStep::Ready(left_item, left_meta), PullStep::Pending(_)) => {
                *this.buffer = Some(Either::Left((left_item, left_meta)));
                PullStep::pending()
            }
            (PullStep::Pending(_), PullStep::Ready(right_item, right_meta)) => {
                *this.buffer = Some(Either::Right((right_item, right_meta)));
                PullStep::pending()
            }
            (PullStep::Pending(_), PullStep::Pending(_)) => PullStep::pending(),
            (PullStep::Pending(_), PullStep::Ended(_)) => PullStep::pending(),
            (PullStep::Ended(_), PullStep::Pending(_)) => PullStep::pending(),
            (PullStep::Ended(_), PullStep::Ended(_)) => PullStep::ended(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (mut min1, mut max1) = self.prev1.size_hint();
        let (mut min2, mut max2) = self.prev2.size_hint();

        // Account for a buffered item: it adds 1 to the respective stream's remaining count.
        match self.buffer {
            Some(Either::Left(_)) => {
                min1 = min1.saturating_add(1);
                max1 = max1.and_then(|m| m.checked_add(1));
            }
            Some(Either::Right(_)) => {
                min2 = min2.saturating_add(1);
                max2 = max2.and_then(|m| m.checked_add(1));
            }
            None => {}
        }

        // Lower bound is the max of the two (we continue until both end)
        let lower = min1.max(min2);
        // Upper bound is the max of the two (if both known)
        let upper = max1.zip(max2).map(|(a, b)| a.max(b));

        (lower, upper)
    }

    fuse_self!();
}

impl<A, B> FusedPull for ZipLongest<A, B>
where
    A: FusedPull,
    B: FusedPull<Meta = A::Meta>,
{
}

#[cfg(test)]
mod tests {
    use core::pin::pin;

    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    use itertools::EitherOrBoth;

    use super::*;
    use crate::pull::test_utils::{TestPull, assert_is_fused};
    use crate::pull::{Pull, PullStep};
    use crate::{No, Yes};

    #[test]
    fn zip_longest_functional_same_length() {
        let mut zip = pin!(ZipLongest::new(
            TestPull::items_fused(0..2),
            TestPull::items_fused(0..2)
        ));
        assert_is_fused(&*zip);
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                PullStep::Ready(item, _) => results.push(item),
                PullStep::Ended(_) => break,
                PullStep::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(
            results,
            vec![EitherOrBoth::Both(0, 0), EitherOrBoth::Both(1, 1)]
        );
    }

    #[test]
    fn zip_longest_functional_first_shorter() {
        let mut zip = pin!(ZipLongest::new(
            TestPull::items_fused(0..1),
            TestPull::items_fused(0..3)
        ));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                PullStep::Ready(item, _) => results.push(item),
                PullStep::Ended(_) => break,
                PullStep::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(
            results,
            vec![
                EitherOrBoth::Both(0, 0),
                EitherOrBoth::Right(1),
                EitherOrBoth::Right(2)
            ]
        );
    }

    #[test]
    fn zip_longest_functional_second_shorter() {
        let mut zip = pin!(ZipLongest::new(
            TestPull::items_fused(0..3),
            TestPull::items_fused(0..1)
        ));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                PullStep::Ready(item, _) => results.push(item),
                PullStep::Ended(_) => break,
                PullStep::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(
            results,
            vec![
                EitherOrBoth::Both(0, 0),
                EitherOrBoth::Left(1),
                EitherOrBoth::Left(2)
            ]
        );
    }

    #[test]
    fn zip_longest_fused_shields_upstream() {
        use crate::pull::test_utils::assert_fused_runtime;

        let p = pin!(ZipLongest::new(
            TestPull::items(0..1).fuse(),
            TestPull::items(0..2).fuse()
        ));
        assert_fused_runtime(p);
    }

    #[test]
    fn zip_longest_size_hint_basic() {
        let mut prev1 = TestPull::<_, _, Yes, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::pending(),
            PullStep::Ready(1, ()),
            PullStep::pending(),
            PullStep::ended(),
        ]);
        let mut prev2 = TestPull::<_, _, Yes, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::pending(),
            PullStep::ended(),
        ]);
        let mut zip = pin!(ZipLongest::new(&mut prev1, &mut prev2));

        assert_eq!(zip.size_hint(), (2, Some(2)));

        // Both Ready → Both(0, 0)
        assert!(matches!(zip.as_mut().pull(&mut ()), PullStep::Ready(..)));
        assert_eq!(zip.size_hint(), (1, Some(1)));

        // Both Pending
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // prev1 Ready(1), prev2 Ended → Left(1)
        assert!(matches!(zip.as_mut().pull(&mut ()), PullStep::Ready(..)));

        // prev1 Pending, prev2 Ended → Pending
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // Both Ended
        assert!(zip.as_mut().pull(&mut ()).is_ended());
    }

    #[test]
    fn zip_longest_size_hint_with_buffered_item() {
        let mut prev1 = TestPull::<_, _, No, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::Ready(1, ()),
            PullStep::Ready(2, ()),
            PullStep::ended(),
        ]);
        let mut prev2 = TestPull::<_, _, Yes, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::pending(),
            PullStep::Ready(1, ()),
            PullStep::pending(),
            PullStep::Ready(2, ()),
            PullStep::pending(),
            PullStep::ended(),
        ]);
        let mut zip = pin!(ZipLongest::new(&mut prev1, &mut prev2));

        // Pull 1: prev1 Ready(0), prev2 Ready(0) => Both(0, 0)
        assert!(matches!(
            zip.as_mut().pull(&mut ()),
            PullStep::Ready(EitherOrBoth::Both(0, 0), ())
        ));

        // Pull 2: prev1 Ready(1), prev2 Pending => buffer Left(1), Pending
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // Now buffer has Left(item1). prev1 reports (1, Some(1)), prev2 reports (2, Some(2)).
        // With buffer: prev1 effective = (2, Some(2)), prev2 = (2, Some(2)) => (2, Some(2))
        assert_eq!(zip.size_hint(), (2, Some(2)));
    }

    #[test]
    fn zip_longest_no_starvation() {
        // When prev1 is Pending, prev2 should still be polled and its item buffered.
        let mut prev1 = TestPull::<_, _, Yes, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::pending(),
            PullStep::Ready(1, ()),
            PullStep::pending(),
            PullStep::ended(),
        ]);
        let mut prev2 = TestPull::<_, _, No, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::Ready(1, ()),
            PullStep::ended(),
        ]);
        let mut zip = pin!(ZipLongest::new(&mut prev1, &mut prev2));

        // prev1 Ready(0), prev2 Ready(0) → Both(0, 0)
        assert!(matches!(
            zip.as_mut().pull(&mut ()),
            PullStep::Ready(EitherOrBoth::Both(0, 0), _)
        ));

        // prev1 Pending, prev2 Ready(1) → buffer Right(1), Pending
        // This proves prev2 was polled even though prev1 was Pending.
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // Buffered Right(1) pairs with prev1 Ready(1) → Both(1, 1)
        assert!(matches!(
            zip.as_mut().pull(&mut ()),
            PullStep::Ready(EitherOrBoth::Both(1, 1), _)
        ));

        // prev1 Pending, prev2 Ended → Pending
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // prev1 Ended, prev2 Ended → Ended
        assert!(zip.as_mut().pull(&mut ()).is_ended());
    }
}
