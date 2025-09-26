use super::{Pusherator, PusheratorBuild};

pub struct Inspect<Next, Func> {
    next: Next,
    func: Func,
}
impl<Next, Func> Pusherator for Inspect<Next, Func>
where
    Next: Pusherator,
    Func: FnMut(&Next::Item),
{
    type Item = Next::Item;

    fn give(&mut self, item: Self::Item) {
        (self.func)(&item);
        self.next.give(item);
    }
}
impl<Next, Func> Inspect<Next, Func>
where
    Next: Pusherator,
    Func: FnMut(&Next::Item),
{
    pub fn new(func: Func, next: Next) -> Self {
        Self { next, func }
    }
}

pub struct InspectBuild<Prev, Func> {
    prev: Prev,
    func: Func,
}
impl<Prev, Func> InspectBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(&Prev::ItemOut),
{
    pub fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}
impl<Prev, Func> PusheratorBuild for InspectBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(&Prev::ItemOut),
{
    type ItemOut = Prev::ItemOut;

    type Output<Next: Pusherator<Item = Self::ItemOut>> = Prev::Output<Inspect<Next, Func>>;
    fn push_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Inspect::new(self.func, next))
    }
}
