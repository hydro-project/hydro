use std::marker::PhantomData;

use super::Pusherator;

#[derive(Clone, Copy)]
pub struct Null<In> {
    phantom: PhantomData<In>,
}

impl<In> Null<In> {
    pub fn new() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

impl<In> Default for Null<In> {
    fn default() -> Self {
        Self::new()
    }
}

impl<In> Pusherator for Null<In> {
    type Item = In;
    fn give(&mut self, _: Self::Item) {}
}
