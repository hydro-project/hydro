use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use super::{CanReceive, Handoff, HandoffMeta, Iter};

/// A [Vec]-based FIFO handoff.
pub struct VecHandoff<T, const LAZY: bool = false>
where
    T: 'static,
{
    pub(crate) input: Rc<RefCell<Vec<T>>>,
    pub(crate) output: Rc<RefCell<Vec<T>>>,
}
impl<T, const LAZY: bool> Default for VecHandoff<T, LAZY>
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
impl<T, const LAZY: bool> Handoff for VecHandoff<T, LAZY> {
    type Inner = Vec<T>;

    fn take_inner(&self) -> Self::Inner {
        self.input.take()
    }

    fn borrow_mut_swap(&self) -> RefMut<Self::Inner> {
        let mut input = self.input.borrow_mut();
        let mut output = self.output.borrow_mut();

        std::mem::swap(&mut *input, &mut *output);

        output
    }
}

impl<T, const LAZY: bool> CanReceive<Option<T>> for VecHandoff<T, LAZY> {
    fn give(&self, mut item: Option<T>) -> Option<T> {
        if let Some(item) = item.take() {
            (*self.input).borrow_mut().push(item)
        }
        None
    }
}
impl<T, I, const LAZY: bool> CanReceive<Iter<I>> for VecHandoff<T, LAZY>
where
    I: Iterator<Item = T>,
{
    fn give(&self, mut iter: Iter<I>) -> Iter<I> {
        (*self.input).borrow_mut().extend(&mut iter.0);
        iter
    }
}
impl<T, const LAZY: bool> CanReceive<Vec<T>> for VecHandoff<T, LAZY> {
    fn give(&self, mut vec: Vec<T>) -> Vec<T> {
        (*self.input).borrow_mut().extend(vec.drain(..));
        vec
    }
}

impl<T, const LAZY: bool> HandoffMeta for VecHandoff<T, LAZY> {
    fn any_ref(&self) -> &dyn Any {
        self
    }

    fn is_bottom(&self) -> bool {
        LAZY || (*self.input).borrow_mut().is_empty()
    }
}

impl<H> HandoffMeta for Rc<RefCell<H>>
where
    H: HandoffMeta,
{
    fn any_ref(&self) -> &dyn Any {
        self
    }

    fn is_bottom(&self) -> bool {
        self.borrow().is_bottom()
    }
}
