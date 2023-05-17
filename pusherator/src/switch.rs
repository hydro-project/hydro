use either::Either;

use super::{Pusherator, PusheratorBuild};

pub struct Switch<Next1, Next2> {
    next1: Next1,
    next2: Next2,
}
impl<Next1, Next2> Pusherator for Switch<Next1, Next2>
where
    Next1: Pusherator,
    Next2: Pusherator,
{
    type Item = Either<Next1::Item, Next2::Item>;
    fn give(&mut self, item: Self::Item) {
        match item {
            Either::Left(item1) => self.next1.give(item1),
            Either::Right(item2) => self.next2.give(item2),
        }
    }
}
impl<Next1, Next2> Switch<Next1, Next2>
where
    Next1: Pusherator,
    Next2: Pusherator,
{
    pub fn new(next1: Next1, next2: Next2) -> Self {
        Self { next1, next2 }
    }
}

pub struct SwitchBuild<Prev, Next1>
where
    Prev: PusheratorBuild,
    Next1: Pusherator,
{
    prev: Prev,
    next1: Next1,
}
impl<Prev, Next1, Item2> SwitchBuild<Prev, Next1>
where
    Prev: PusheratorBuild<ItemOut = Either<Next1::Item, Item2>>,
    Next1: Pusherator,
{
    pub fn new(prev: Prev, next1: Next1) -> Self {
        Self { prev, next1 }
    }
}
impl<Prev, Next1, Item2> PusheratorBuild for SwitchBuild<Prev, Next1>
where
    Prev: PusheratorBuild<ItemOut = Either<Next1::Item, Item2>>,
    Next1: Pusherator,
{
    type ItemOut = Item2;

    type Output<Next: Pusherator<Item = Self::ItemOut>> = Prev::Output<Switch<Next1, Next>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Switch::new(self.next1, input))
    }
}
