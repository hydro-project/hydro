use core::pin::Pin;

use itertools::Either;
use pin_project_lite::pin_project;

use crate::pull::{Pull, PullStep};
use crate::{Context, Toggle};

pin_project! {
    /// A pull that zips two pulls together, ending when either is exhausted.
    ///
    /// Yields `(Item1, Item2)` pairs. Ends as soon as either upstream pull ends.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct Zip<Prev1, Prev2>
    where
        Prev1: Pull,
        Prev2: Pull,
    {
        #[pin]
        prev1: Prev1,
        #[pin]
        prev2: Prev2,
        // Buffer an item from whichever stream was ready first, to prevent starvation.
        buffer: Option<Either<(Prev1::Item, Prev1::Meta), (Prev2::Item, Prev2::Meta)>>,
    }
}

impl<Prev1, Prev2> Zip<Prev1, Prev2>
where
    Prev1: Pull,
    Prev2: Pull,
    Self: Pull,
{
    /// Create a new `Zip` stream from two source streams.
    pub(crate) const fn new(prev1: Prev1, prev2: Prev2) -> Self {
        Self {
            prev1,
            prev2,
            buffer: None,
        }
    }
}

impl<Prev1, Prev2> Pull for Zip<Prev1, Prev2>
where
    Prev1: Pull,
    Prev2: Pull<Meta = Prev1::Meta>,
{
    type Ctx<'ctx> = <Prev1::Ctx<'ctx> as Context<'ctx>>::Merged<Prev2::Ctx<'ctx>>;

    type Item = (Prev1::Item, Prev2::Item);
    type Meta = Prev1::Meta;
    type CanPend = <Prev1::CanPend as Toggle>::Or<Prev2::CanPend>;
    type CanEnd = <Prev1::CanEnd as Toggle>::Or<Prev2::CanEnd>;

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
                PullStep::Ready((left_item, right_item), left_meta)
            } // TODO(mingwei): use right_meta
            (PullStep::Ready(left_item, left_meta), PullStep::Pending(_)) => {
                *this.buffer = Some(Either::Left((left_item, left_meta)));
                PullStep::pending()
            }
            (PullStep::Pending(_), PullStep::Ready(right_item, right_meta)) => {
                *this.buffer = Some(Either::Right((right_item, right_meta)));
                PullStep::pending()
            }
            (PullStep::Pending(_), PullStep::Pending(_)) => PullStep::pending(),
            // Any Ended → whole zip ends.
            (PullStep::Ready(..), PullStep::Ended(_))
            | (PullStep::Ended(_), PullStep::Ready(..))
            | (PullStep::Pending(_), PullStep::Ended(_))
            | (PullStep::Ended(_), PullStep::Pending(_))
            | (PullStep::Ended(_), PullStep::Ended(_)) => PullStep::ended(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (mut min1, mut max1) = self.prev1.size_hint();
        let (mut min2, mut max2) = self.prev2.size_hint();

        // Account for the buffered item: if we have a buffered item from one side,
        // that side effectively has one more item remaining.
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

        // Lower bound is the min of the two (we end when either ends)
        let lower = min1.min(min2);
        // Upper bound is the min of the two (if either known)
        let upper = match (max1, max2) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };

        (lower, upper)
    }
}

#[cfg(test)]
mod tests {
    use core::pin::pin;

    extern crate alloc;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;
    use crate::pull::test_utils::TestPull;
    use crate::pull::{Pull, PullStep};
    use crate::{No, Yes};

    #[test]
    fn zip_functional_same_length() {
        let mut zip = pin!(Zip::new(TestPull::items(0..2), TestPull::items(0..2)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                PullStep::Ready(item, _) => results.push(item),
                PullStep::Ended(_) => break,
                PullStep::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(results, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn zip_functional_first_shorter() {
        let mut zip = pin!(Zip::new(TestPull::items(0..1), TestPull::items(0..3)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                PullStep::Ready(item, _) => results.push(item),
                PullStep::Ended(_) => break,
                PullStep::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(results, vec![(0, 0)]);
    }

    #[test]
    fn zip_functional_second_shorter() {
        let mut zip = pin!(Zip::new(TestPull::items(0..3), TestPull::items(0..1)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                PullStep::Ready(item, _) => results.push(item),
                PullStep::Ended(_) => break,
                PullStep::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(results, vec![(0, 0)]);
    }

    #[test]
    fn zip_size_hint_includes_buffer() {
        // After one pair consumed and prev1's item buffered as Left,
        // prev1's raw size_hint is (0, Some(0)) but the buffer adds 1,
        // making the effective hint (1, Some(1)) instead of (0, Some(0)).
        let mut prev1 = TestPull::<_, _, No, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::Ready(1, ()),
            PullStep::ended(),
        ]);
        let mut prev2 = TestPull::<_, _, Yes, Yes, true>::new([
            PullStep::Ready(0, ()),
            PullStep::pending(),
            PullStep::Ready(1, ()),
            PullStep::pending(),
            PullStep::ended(),
        ]);
        let mut zip = pin!(Zip::new(&mut prev1, &mut prev2));

        // Initial: prev1=(2,Some(2)), prev2=(2,Some(2)) → min = (2, Some(2))
        assert_eq!(zip.size_hint(), (2, Some(2)));

        // Pull 1: prev1 Ready(0), prev2 Ready(0) → Ready((0,0))
        assert_eq!(zip.as_mut().pull(&mut ()), PullStep::Ready((0, 0), ()));

        // Pull 2: prev1 Ready(1), prev2 Pending → buffer Left(1), Pending
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // prev1 raw=(0,Some(0)) + buffer Left → effective (1,Some(1))
        // prev2 raw=(1,Some(1))
        // Without buffer accounting this would be (0, Some(0)).
        assert_eq!(zip.size_hint(), (1, Some(1)));
    }

    #[test]
    fn zip_no_starvation() {
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
        let mut zip = pin!(Zip::new(&mut prev1, &mut prev2));

        // Pull 1: both Ready → Ready((0, 0))
        assert!(matches!(
            zip.as_mut().pull(&mut ()),
            PullStep::Ready((0, 0), _)
        ));

        // Pull 2: prev1 Pending, prev2 Ready(1) → buffered as Right, Pending
        assert!(zip.as_mut().pull(&mut ()).is_pending());

        // Pull 3: prev1 Ready(1) pairs with buffered Right(1) → Ready((1, 1))
        // This proves prev2 was polled (not starved) when prev1 was Pending.
        assert!(matches!(
            zip.as_mut().pull(&mut ()),
            PullStep::Ready((1, 1), _)
        ));
    }
}
