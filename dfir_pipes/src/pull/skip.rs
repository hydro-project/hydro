use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep};

pin_project! {
    /// Pull combinator that skips the first `n` items.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Skip<Prev> {
        #[pin]
        prev: Prev,
        remaining: usize,
    }
}

impl<Prev> Skip<Prev>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, n: usize) -> Self {
        Self { prev, remaining: n }
    }
}

impl<Prev> Pull for Skip<Prev>
where
    Prev: Pull,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Prev::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Prev::CanEnd;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let mut this = self.project();

        loop {
            return match this.prev.as_mut().pull(ctx) {
                PullStep::Ready(item, meta) => {
                    if *this.remaining > 0 {
                        *this.remaining -= 1;
                        continue;
                    }
                    PullStep::Ready(item, meta)
                }
                PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
                PullStep::Ended(can_end) => PullStep::Ended(can_end),
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.prev.size_hint();
        let remaining = self.remaining;
        (
            lower.saturating_sub(remaining),
            upper.map(|u| u.saturating_sub(remaining)),
        )
    }
}

impl<Prev> FusedPull for Skip<Prev> where Prev: FusedPull {}
