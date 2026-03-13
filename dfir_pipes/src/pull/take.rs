use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::Yes;
use crate::pull::{FusedPull, Pull, PullStep, fuse_self};

pin_project! {
    /// Pull combinator that yields the first `n` items.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    pub struct Take<Prev> {
        #[pin]
        prev: Prev,
        remaining: usize,
    }
}

impl<Prev> Take<Prev>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, n: usize) -> Self {
        Self { prev, remaining: n }
    }
}

impl<Prev> Pull for Take<Prev>
where
    Prev: Pull,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Prev::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Yes;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.project();

        if 0 == *this.remaining {
            return PullStep::Ended(Yes);
        }

        match this.prev.pull(ctx) {
            PullStep::Ready(item, meta) => {
                *this.remaining -= 1;
                PullStep::Ready(item, meta)
            }
            PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
            PullStep::Ended(_) => {
                *this.remaining = 0;
                PullStep::Ended(Yes)
            }
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        let (lower, upper) = this.prev.size_hint();
        let remaining = *this.remaining;
        (
            lower.min(remaining),
            upper.map(|u| u.min(remaining)).or(Some(remaining)),
        )
    }

    fuse_self!();
}

impl<Prev> FusedPull for Take<Prev> where Prev: Pull {}

#[cfg(test)]
mod tests {
    use core::pin::pin;

    use crate::pull::Pull;
    use crate::pull::test_utils::{PanicsAfterEndPull, assert_fused_runtime};

    #[test]
    fn take_fused_shields_upstream() {
        let p = pin!(PanicsAfterEndPull::new(2).take(1));
        assert_fused_runtime(p);
    }
}
