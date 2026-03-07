use core::pin::Pin;

use itertools::EitherOrBoth;
use pin_project_lite::pin_project;

use crate::{Context, FusedPull, Pull, Step, Toggle, fuse_self};

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
        Prev1: Pull
    {
        #[pin]
        prev1: Prev1,
        #[pin]
        prev2: Prev2,
        // Store the first stream's item when the second stream is not ready.
        item1: Option<(Prev1::Item, Prev1::Meta)>,
    }
}

impl<Prev1, Prev2> ZipLongest<Prev1, Prev2>
where
    Prev1: Pull,
    Self: Pull,
{
    /// Create a new `ZipLongest` stream from two source streams.
    pub(crate) const fn new(prev1: Prev1, prev2: Prev2) -> Self {
        Self {
            prev1,
            prev2,
            item1: None,
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        // Store `item1` so it is not dropped if `stream2` returns `Poll::Pending`.
        if this.item1.is_none() {
            *this.item1 = match this
                .prev1
                .as_mut()
                .pull(<Prev1::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
            {
                Step::Ready(item, meta) => Some((item, meta)),
                Step::Pending(_) => {
                    return Step::pending();
                }
                Step::Ended(_) => None,
            };
        }
        let item2 = this
            .prev2
            .as_mut()
            .pull(<Prev1::Ctx<'_> as Context<'_>>::unmerge_other(ctx));
        if let Step::Pending(_) = item2 {
            return Step::pending();
        }

        match (this.item1.take(), item2) {
            (_, Step::Pending(_)) => unreachable!(),
            (None, Step::Ready(item2, meta2)) => Step::Ready(EitherOrBoth::Right(item2), meta2),
            (None, Step::Ended(_)) => Step::ended(),
            (Some((item1, meta1)), Step::Ready(item2, _meta2)) => {
                Step::Ready(EitherOrBoth::Both(item1, item2), meta1)
            } // TODO(mingwei): use _meta2
            (Some((item1, meta1)), Step::Ended(_)) => Step::Ready(EitherOrBoth::Left(item1), meta1),
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();

        let (min1, max1) = this.prev1.size_hint();
        let (min2, max2) = this.prev2.size_hint();

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

    use super::ZipLongest;
    use crate::test_utils::SyncPull;
    use crate::{Pull, Step};

    #[test]
    fn zip_longest_functional_same_length() {
        let mut zip = pin!(ZipLongest::new(SyncPull::new(2), SyncPull::new(2)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => results.push(item),
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(
            results,
            vec![EitherOrBoth::Both(0, 0), EitherOrBoth::Both(1, 1)]
        );
    }

    #[test]
    fn zip_longest_functional_first_shorter() {
        let mut zip = pin!(ZipLongest::new(SyncPull::new(1), SyncPull::new(3)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => results.push(item),
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
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
        let mut zip = pin!(ZipLongest::new(SyncPull::new(3), SyncPull::new(1)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => results.push(item),
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
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
}
