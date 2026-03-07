use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{Context, FusedPull, Pull, Step, Toggle};

pin_project! {
    /// A pull that zips two pulls together, ending when either is exhausted.
    ///
    /// Yields `(Item1, Item2)` pairs. Ends as soon as either upstream pull ends.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct Zip<Prev1, Prev2>
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

impl<Prev1, Prev2> Zip<Prev1, Prev2>
where
    Prev1: Pull,
    Self: Pull,
{
    /// Create a new `Zip` stream from two source streams.
    pub(crate) const fn new(prev1: Prev1, prev2: Prev2) -> Self {
        Self {
            prev1,
            prev2,
            item1: None,
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        // Store `item1` so it is not dropped if `prev2` returns `Pending`.
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
                Step::Ended(_) => {
                    return Step::ended();
                }
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
            (Some((item1, meta1)), Step::Ready(item2, _meta2)) => {
                Step::Ready((item1, item2), meta1)
            } // TODO(mingwei): use _meta2
            (_, Step::Pending(_)) => Step::pending(),
            (_, Step::Ended(_)) => Step::ended(),
            (None, Step::Ready(_, _)) => {
                unreachable!("item1 is always Some when reaching this match")
            }
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();

        let (min1, max1) = this.prev1.size_hint();
        let (min2, max2) = this.prev2.size_hint();

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

impl<A, B> FusedPull for Zip<A, B>
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

    use super::Zip;
    use crate::test_utils::SyncPull;
    use crate::{Pull, Step};

    #[test]
    fn zip_functional_same_length() {
        let mut zip = pin!(Zip::new(SyncPull::new(2), SyncPull::new(2)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => results.push(item),
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(results, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn zip_functional_first_shorter() {
        let mut zip = pin!(Zip::new(SyncPull::new(1), SyncPull::new(3)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => results.push(item),
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(results, vec![(0, 0)]);
    }

    #[test]
    fn zip_functional_second_shorter() {
        let mut zip = pin!(Zip::new(SyncPull::new(3), SyncPull::new(1)));
        let mut results = Vec::new();

        loop {
            match zip.as_mut().pull(&mut ()) {
                Step::Ready(item, _) => results.push(item),
                Step::Ended(_) => break,
                Step::Pending(_) => unreachable!(),
            }
        }

        assert_eq!(results, vec![(0, 0)]);
    }
}
