use super::{Pusherator, PusheratorBuild};

pub struct Partition<Next1, Next2, Func> {
    next1: Next1,
    next2: Next2,
    func: Func,
}
impl<Next1, Next2, Func> Pusherator for Partition<Next1, Next2, Func>
where
    Next1: Pusherator,
    Next2: Pusherator<Item = Next1::Item>,
    Func: FnMut(&Next1::Item) -> bool,
{
    type Item = Next1::Item;
    fn give(&mut self, item: Self::Item) {
        if (self.func)(&item) {
            self.next1.give(item);
        } else {
            self.next2.give(item);
        }
    }
}
impl<Next1, Next2, Func> Partition<Next1, Next2, Func>
where
    Next1: Pusherator,
    Next2: Pusherator<Item = Next1::Item>,
    Func: FnMut(&Next1::Item) -> bool,
{
    pub fn new(func: Func, next1: Next1, next2: Next2) -> Self {
        Self { next1, next2, func }
    }
}

pub struct PartitionBuild<Prev, Next1, Func>
where
    Prev: PusheratorBuild,
    Next1: Pusherator<Item = Prev::ItemOut>,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    prev: Prev,
    next1: Next1,
    func: Func,
}
impl<Prev, Next1, Func> PartitionBuild<Prev, Next1, Func>
where
    Prev: PusheratorBuild,
    Next1: Pusherator<Item = Prev::ItemOut>,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    pub fn new(prev: Prev, next1: Next1, func: Func) -> Self {
        Self { prev, next1, func }
    }
}
impl<Prev, Next1, Func> PusheratorBuild for PartitionBuild<Prev, Next1, Func>
where
    Prev: PusheratorBuild,
    Next1: Pusherator<Item = Prev::ItemOut>,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    type ItemOut = Prev::ItemOut;

    type Output<Next: Pusherator<Item = Self::ItemOut>> =
        Prev::Output<Partition<Next1, Next, Func>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev
            .push_to(Partition::new(self.func, self.next1, input))
    }
}
