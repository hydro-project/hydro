use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use super::{CanReceive, Handoff, HandoffMeta, Iter};

/// A [Vec]-based FIFO handoff.
pub struct VecHandoff<T>
where
    T: 'static,
{
    pub(crate) input: Rc<RefCell<Vec<T>>>,
    pub(crate) output: Rc<RefCell<Vec<T>>>,
}
impl<T> Default for VecHandoff<T>
where
    T: 'static,
{
    fn default() -> Self {
        Self {
            input: Default::default(),
            output: Default::default(),
        }
    }
}
impl<T> Handoff for VecHandoff<T> {
    type Inner = Vec<T>;

    fn take_inner(&self) -> Self::Inner {
        self.input.take()
    }

    fn borrow_mut_swap(&self) -> RefMut<'_, Self::Inner> {
        let mut input = self.input.borrow_mut();
        let mut output = self.output.borrow_mut();

        std::mem::swap(&mut *input, &mut *output);

        output
    }
}

impl<T> CanReceive<Option<T>> for VecHandoff<T> {
    fn give(&self, mut item: Option<T>) -> Option<T> {
        if let Some(item) = item.take() {
            (*self.input).borrow_mut().push(item)
        }
        None
    }
}
impl<T, I> CanReceive<Iter<I>> for VecHandoff<T>
where
    I: Iterator<Item = T>,
{
    fn give(&self, mut iter: Iter<I>) -> Iter<I> {
        (*self.input).borrow_mut().extend(&mut iter.0);
        iter
    }
}
impl<T> CanReceive<Vec<T>> for VecHandoff<T> {
    fn give(&self, mut vec: Vec<T>) -> Vec<T> {
        (*self.input).borrow_mut().extend(vec.drain(..));
        vec
    }
}

impl<T> HandoffMeta for VecHandoff<T> {
    fn len(&self) -> usize {
        (*self.input).borrow().len()
    }
}
