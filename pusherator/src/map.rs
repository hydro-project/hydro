use std::marker::PhantomData;

use super::{Pusherator, PusheratorBuild};

pub struct Map<Next, Func, In> {
    next: Next,
    func: Func,
    _in: PhantomData<fn(In)>,
}
impl<Next, Func, In> Pusherator for Map<Next, Func, In>
where
    Next: Pusherator,
    Func: FnMut(In) -> Next::Item,
{
    type Item = In;
    fn give(&mut self, item: Self::Item) {
        self.next.give((self.func)(item));
    }
}
impl<Next, Func, In> Map<Next, Func, In>
where
    Next: Pusherator,
    Func: FnMut(In) -> Next::Item,
{
    pub fn new(func: Func, next: Next) -> Self {
        Self {
            next,
            func,
            _in: PhantomData,
        }
    }
}

pub struct MapBuild<Prev, Func> {
    prev: Prev,
    func: Func,
}
impl<Prev, Func, Out> MapBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(Prev::ItemOut) -> Out,
{
    pub fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}
impl<Prev, Func, Out> PusheratorBuild for MapBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(Prev::ItemOut) -> Out,
{
    type ItemOut = Out;

    type Output<Next: Pusherator<Item = Self::ItemOut>> =
        Prev::Output<Map<Next, Func, Prev::ItemOut>>;
    fn push_to<Next>(self, next: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Map::new(self.func, next))
    }
}
