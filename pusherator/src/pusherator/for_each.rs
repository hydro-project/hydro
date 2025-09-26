use std::marker::PhantomData;

use super::Pusherator;

pub struct ForEach<Func, In> {
    func: Func,
    _marker: PhantomData<fn(In)>,
}
impl<Func, In> Pusherator for ForEach<Func, In>
where
    Func: FnMut(In),
{
    type Item = In;
    fn give(&mut self, item: Self::Item) {
        (self.func)(item)
    }
}
impl<Func, In> ForEach<Func, In>
where
    Func: FnMut(In),
{
    pub fn new(func: Func) -> Self {
        Self {
            func,
            _marker: PhantomData,
        }
    }
}
