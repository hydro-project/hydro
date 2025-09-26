use super::{Pusherator, PusheratorBuild};

pub struct Tee<Next1, Next2> {
    next1: Next1,
    next2: Next2,
}
impl<Next1, Next2> Pusherator for Tee<Next1, Next2>
where
    Next1: Pusherator,
    Next2: Pusherator<Item = Next1::Item>,
    Next1::Item: Clone,
{
    type Item = Next1::Item;
    fn give(&mut self, item: Self::Item) {
        self.next1.give(item.clone());
        self.next2.give(item);
    }
}
impl<Next1, Next2> Tee<Next1, Next2>
where
    Next1: Pusherator,
    Next2: Pusherator<Item = Next1::Item>,
    Next1::Item: Clone,
{
    pub fn new(next1: Next1, next2: Next2) -> Self {
        Self { next1, next2 }
    }
}

pub struct TeeBuild<Prev, Next1>
where
    Prev: PusheratorBuild,
    Next1: Pusherator<Item = Prev::ItemOut>,
    Prev::ItemOut: Clone,
{
    prev: Prev,
    next1: Next1,
}
impl<Prev, Next1> TeeBuild<Prev, Next1>
where
    Prev: PusheratorBuild,
    Next1: Pusherator<Item = Prev::ItemOut>,
    Prev::ItemOut: Clone,
{
    pub fn new(prev: Prev, next1: Next1) -> Self {
        Self { prev, next1 }
    }
}
impl<Prev, Next1> PusheratorBuild for TeeBuild<Prev, Next1>
where
    Prev: PusheratorBuild,
    Next1: Pusherator<Item = Prev::ItemOut>,
    Prev::ItemOut: Clone,
{
    type ItemOut = Prev::ItemOut;

    type Output<Next: Pusherator<Item = Self::ItemOut>> = Prev::Output<Tee<Next1, Next>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Tee::new(self.next1, input))
    }
}
