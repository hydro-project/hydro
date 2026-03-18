use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::pull::{FusedPull, Pull, PullStep, fuse_self};

pin_project! {
    /// Pull combinator that guarantees [`PullStep::Ended`] is returned forever after the first end.
    #[must_use = "`Pull`s do nothing unless polled"]
    #[derive(Clone, Debug, Default)]
    #[project_replace = FuseReplace]
    pub struct Fuse<Prev> {
        #[pin]
        prev: Option<Prev>,
    }
}

impl<Prev> Fuse<Prev>
where
    Self: Pull,
{
    pub(crate) const fn new(prev: Prev) -> Self {
        Self { prev: Some(prev) }
    }
}

impl<Prev> Pull for Fuse<Prev>
where
    Prev: Pull,
{
    type Ctx<'ctx> = Prev::Ctx<'ctx>;

    type Item = Prev::Item;
    type Meta = Prev::Meta;
    type CanPend = Prev::CanPend;
    type CanEnd = Prev::CanEnd;

    fn pull(
        mut self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.as_mut().project();

        if let Some(prev) = this.prev.as_pin_mut() {
            match prev.pull(ctx) {
                PullStep::Ready(item, meta) => PullStep::Ready(item, meta),
                PullStep::Pending(can_pend) => PullStep::Pending(can_pend),
                PullStep::Ended(_) => {
                    let _ = self.project_replace(Self { prev: None });
                    PullStep::ended()
                }
            }
        } else {
            PullStep::ended()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(prev) = self.prev.as_ref() {
            prev.size_hint()
        } else {
            (0, Some(0))
        }
    }

    fuse_self!();
}

impl<Prev> FusedPull for Fuse<Prev> where Prev: Pull {}

#[cfg(test)]
mod tests {
    use core::pin::pin;

    use crate::pull::Pull;
    use crate::pull::test_utils::{TestPull, assert_fused_runtime};

    #[test]
    fn fuse_shields_upstream() {
        let p = pin!(TestPull::items(0..2).fuse());
        assert_fused_runtime(p);
    }
}
