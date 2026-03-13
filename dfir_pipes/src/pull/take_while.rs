use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::Yes;
use crate::pull::{FusedPull, Pull, PullStep, fuse_self};

pin_project! {
    /// Pull combinator that yields items while a predicate returns `true`.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug)]
    pub struct TakeWhile<Prev, Func> {
        #[pin]
        prev: Prev,
        func: Func,
        done: bool,
    }
}

impl<Prev, Func> TakeWhile<Prev, Func>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev, func: Func) -> Self {
        Self {
            prev,
            func,
            done: false,
        }
    }
}

impl<Prev, Func> Pull for TakeWhile<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(&Prev::Item) -> bool,
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

        if *this.done {
            return PullStep::Ended(Yes);
        }

        match this.prev.pull(ctx) {
            PullStep::Ready(item, meta) => {
                if (this.func)(&item) {
                    PullStep::Ready(item, meta)
                } else {
                    *this.done = true;
                    PullStep::Ended(Yes)
                }
            }
            PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
            PullStep::Ended(_) => {
                *this.done = true;
                PullStep::Ended(Yes)
            }
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        if *this.done {
            (0, Some(0))
        } else {
            let (_, upper) = this.prev.size_hint();
            (0, upper)
        }
    }

    fuse_self!();
}

impl<Prev, Func> FusedPull for TakeWhile<Prev, Func>
where
    Prev: Pull,
    Func: FnMut(&Prev::Item) -> bool,
{
}

#[cfg(test)]
mod tests {
    use core::pin::pin;

    use crate::pull::Pull;
    use crate::pull::test_utils::{PanicsAfterEndPull, assert_fused_runtime};

    #[test]
    fn take_while_fused_shields_upstream() {
        let p = pin!(PanicsAfterEndPull::new(2).take_while(|x| *x < 1));
        assert_fused_runtime(p);
    }
}
