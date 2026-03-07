use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::{FusedPull, Pull, Step, fuse_self};

pin_project! {
    /// Pull combinator that guarantees [`Step::Ended`] is returned forever after the first end.
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
    ) -> Step<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        let this = self.as_mut().project();

        if let Some(prev) = this.prev.as_pin_mut() {
            match prev.pull(ctx) {
                Step::Ready(item, meta) => Step::Ready(item, meta),
                Step::Pending(can_pend) => Step::Pending(can_pend),
                Step::Ended(_) => {
                    let _ = self.project_replace(Self { prev: None });
                    Step::ended()
                }
            }
        } else {
            Step::ended()
        }
    }

    fn size_hint(self: Pin<&Self>) -> (usize, Option<usize>) {
        let this = self.project_ref();
        if let Some(prev) = this.prev.as_pin_ref() {
            prev.size_hint()
        } else {
            (0, Some(0))
        }
    }

    fuse_self!();
}

impl<Prev> FusedPull for Fuse<Prev> where Prev: Pull {}
