use core::pin::Pin;

use itertools::Either;

use crate::pull::{FusedPull, Pull, PullStep};
use crate::{Context, Toggle};

impl<L, R> Pull for Either<L, R>
where
    L: Pull,
    R: Pull<Item = L::Item, Meta = L::Meta>,
{
    type Ctx<'ctx> = <L::Ctx<'ctx> as Context<'ctx>>::Merged<R::Ctx<'ctx>>;

    type Item = L::Item;
    type Meta = L::Meta;
    type CanPend = <L::CanPend as Toggle>::Or<R::CanPend>;
    type CanEnd = <L::CanEnd as Toggle>::Or<R::CanEnd>;

    fn pull(
        self: Pin<&mut Self>,
        ctx: &mut Self::Ctx<'_>,
    ) -> PullStep<Self::Item, Self::Meta, Self::CanPend, Self::CanEnd> {
        match self.as_pin_mut() {
            Either::Left(left) => left
                .pull(<L::Ctx<'_> as Context<'_>>::unmerge_self(ctx))
                .convert_into(),
            Either::Right(right) => right
                .pull(<L::Ctx<'_> as Context<'_>>::unmerge_other(ctx))
                .convert_into(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.as_ref() {
            Either::Left(left) => left.size_hint(),
            Either::Right(right) => right.size_hint(),
        }
    }
}

impl<L, R> FusedPull for Either<L, R>
where
    L: FusedPull,
    R: FusedPull<Item = L::Item, Meta = L::Meta>,
{
}
