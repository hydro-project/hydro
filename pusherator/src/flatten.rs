use std::marker::PhantomData;

use super::{Pusherator, PusheratorBuild};

pub struct Flatten<Next, In> {
    next: Next,
    _marker: PhantomData<fn(In)>,
}
impl<Next, In> Pusherator for Flatten<Next, In>
where
    Next: Pusherator<Item = In::Item>,
    In: IntoIterator,
{
    type Item = In;
    fn give(&mut self, item: Self::Item) {
        for x in item {
            self.next.give(x);
        }
    }
}
impl<Next, In> Flatten<Next, In>
where
    Next: Pusherator<Item = In::Item>,
    In: IntoIterator,
{
    pub fn new(next: Next) -> Self {
        Self {
            next,
            _marker: PhantomData,
        }
    }
}

pub struct FlattenBuild<Prev>
where
    Prev: PusheratorBuild,
    Prev::ItemOut: IntoIterator,
{
    prev: Prev,
}
impl<Prev> FlattenBuild<Prev>
where
    Prev: PusheratorBuild,
    Prev::ItemOut: IntoIterator,
{
    pub fn new(prev: Prev) -> Self {
        Self { prev }
    }
}
impl<Prev> PusheratorBuild for FlattenBuild<Prev>
where
    Prev: PusheratorBuild,
    Prev::ItemOut: IntoIterator,
{
    type ItemOut = <Prev::ItemOut as IntoIterator>::Item;

    type Output<Next: Pusherator<Item = Self::ItemOut>> =
        Prev::Output<Flatten<Next, Prev::ItemOut>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>,
    {
        self.prev.push_to(Flatten::new(input))
    }
}
