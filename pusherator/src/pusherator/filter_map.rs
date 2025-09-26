use std::marker::PhantomData;

use super::{Pusherator, PusheratorBuild};

pub struct FilterMap<Next, Func, In> {
    next: Next,
    func: Func,
    _marker: PhantomData<fn(In)>,
}
impl<Next, Func, In> Pusherator for FilterMap<Next, Func, In>
where
    Next: Pusherator,
    Func: FnMut(In) -> Option<Next::Item>,
{
    type Item = In;
    fn give(&mut self, item: Self::Item) {
        if let Some(item) = (self.func)(item) {
            self.next.give(item);
        }
    }
}
impl<Next, Func, In> FilterMap<Next, Func, In>
where
    Next: Pusherator,
    Func: FnMut(In) -> Option<Next::Item>,
{
    pub fn new(func: Func, next: Next) -> Self {
        Self {
            next,
            func,
            _marker: PhantomData,
        }
    }
}

pub struct FilterMapBuild<Prev, Func>
where
    Prev: PusheratorBuild,
{
    prev: Prev,
    func: Func,
}
impl<Prev, Func, Out> FilterMapBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(Prev::ItemOut) -> Option<Out>,
{
    pub fn new(prev: Prev, func: Func) -> Self {
        Self { prev, func }
    }
}
impl<Prev, Func, Out> PusheratorBuild for FilterMapBuild<Prev, Func>
where
    Prev: PusheratorBuild,
    Func: FnMut(Prev::ItemOut) -> Option<Out>,
{
    type ItemOut = Out;

    type Output<O: Pusherator<Item = Self::ItemOut>> =
        Prev::Output<FilterMap<O, Func, Prev::ItemOut>>;
    fn push_to<O>(self, input: O) -> Self::Output<O>
    where
        O: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(FilterMap::new(self.func, input))
    }
}
