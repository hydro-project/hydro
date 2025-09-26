use super::{Pusherator, PusheratorBuild};

pub struct Filter<Next, Func> {
    next: Next,
    func: Func,
}
impl<Next, Func> Pusherator for Filter<Next, Func>
where
    Next: Pusherator,
    Func: FnMut(&Next::Item) -> bool,
{
    type Item = Next::Item;
    fn give(&mut self, item: Self::Item) {
        if (self.func)(&item) {
            self.next.give(item);
        }
    }
}
impl<Next, Func> Filter<Next, Func>
where
    Next: Pusherator,
    Func: FnMut(&Next::Item) -> bool,
{
    pub fn new(func: Func, next: Next) -> Self {
        Self { next, func }
    }
}

pub struct FilterBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    prev: Prev,
    func: Func,
}
impl<Prev, Func> FilterBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    pub fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}
impl<Prev, Func> PusheratorBuild for FilterBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(&Prev::ItemOut) -> bool,
{
    type ItemOut = Prev::ItemOut;

    type Output<Next: Pusherator<Item = Self::ItemOut>> = Prev::Output<Filter<Next, Func>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Filter::new(self.func, input))
    }
}
