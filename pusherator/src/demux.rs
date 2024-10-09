use std::marker::PhantomData;

use super::Pusherator;

pub struct Demux<Func, Nexts, Item> {
    func: Func,
    nexts: Nexts,
    _phantom: PhantomData<fn(Item)>,
}
impl<Func, Nexts, Item> Pusherator for Demux<Func, Nexts, Item>
where
    Func: FnMut(Item, &mut Nexts),
{
    type Item = Item;
    fn give(&mut self, item: Self::Item) {
        (self.func)(item, &mut self.nexts);
    }
}
impl<Func, Nexts, Item> Demux<Func, Nexts, Item>
where
    Func: FnMut(Item, &mut Nexts),
{
    pub fn new(func: Func, nexts: Nexts) -> Self {
        Self {
            func,
            nexts,
            _phantom: PhantomData,
        }
    }
}
