use super::{Pusherator, PusheratorBuild};

pub struct Unzip<Next1, Next2> {
    next1: Next1,
    next2: Next2,
}
impl<Next1, Next2> Pusherator for Unzip<Next1, Next2>
where
    Next1: Pusherator,
    Next2: Pusherator,
{
    type Item = (Next1::Item, Next2::Item);
    fn give(&mut self, (item1, item2): Self::Item) {
        self.next1.give(item1);
        self.next2.give(item2);
    }
}
impl<Next1, Next2> Unzip<Next1, Next2>
where
    Next1: Pusherator,
    Next2: Pusherator,
{
    pub fn new(next1: Next1, next2: Next2) -> Self {
        Self { next1, next2 }
    }
}

pub struct UnzipBuild<Prev, Next1>
where
    Prev: PusheratorBuild,
    Next1: Pusherator,
{
    prev: Prev,
    next1: Next1,
}
impl<Prev, Next1, Item2> UnzipBuild<Prev, Next1>
where
    Prev: PusheratorBuild<ItemOut = (Next1::Item, Item2)>,
    Next1: Pusherator,
{
    pub fn new(prev: Prev, next1: Next1) -> Self {
        Self { prev, next1 }
    }
}
impl<Prev, Next1, Item2> PusheratorBuild for UnzipBuild<Prev, Next1>
where
    Prev: PusheratorBuild<ItemOut = (Next1::Item, Item2)>,
    Next1: Pusherator,
{
    type ItemOut = Item2;

    type Output<Next: Pusherator<Item = Self::ItemOut>> = Prev::Output<Unzip<Next1, Next>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Unzip::new(self.next1, input))
    }
}
