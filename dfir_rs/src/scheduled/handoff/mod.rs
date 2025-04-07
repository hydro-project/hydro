//! Module for all [`Handoff`]-related items.

pub mod handoff_list;
mod tee;
mod vector;

use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::rc::Rc;

pub use tee::TeeingHandoff;
pub use vector::VecHandoff;

/// Trait representing something which we can attempt to give an item to.
pub trait TryCanReceive<T> {
    // TODO(mingwei): Isn't used.
    /// Try to give a value to the handoff, may return an error if full, representing backpressure.
    fn try_give(&self, item: T) -> Result<T, T>;
}

/// Trait representing somethign which we can give an item to.
pub trait CanReceive<T> {
    // TODO: represent backpressure in this return value.
    /// Give a value to the handoff.
    fn give(&self, item: T) -> T;
}

/// A handle onto the metadata part of a [Handoff], with no element type.
pub trait HandoffMeta: Any {
    // TODO(justin): more fine-grained info here.
    /// Return if the handoff is empty.
    fn is_bottom(&self) -> bool;
}

impl<H> HandoffMeta for Rc<RefCell<H>>
where
    H: HandoffMeta,
{
    fn is_bottom(&self) -> bool {
        self.borrow().is_bottom()
    }
}

/// Trait for handoffs to implement.
pub trait Handoff: Default + HandoffMeta {
    /// Inner datastructure type.
    type Inner;

    /// Take the inner datastructure, similar to [`std::mem::take`].
    fn take_inner(&self) -> Self::Inner;

    /// Take the inner datastructure by swapping input and output buffers.
    ///
    /// For better performance over [`Self::take_inner`].
    fn borrow_mut_swap(&self) -> RefMut<Self::Inner>;

    /// See [`CanReceive::give`].
    fn give<T>(&self, item: T) -> T
    where
        Self: CanReceive<T>,
    {
        <Self as CanReceive<T>>::give(self, item)
    }

    /// See [`TryCanReceive::try_give`].
    fn try_give<T>(&self, item: T) -> Result<T, T>
    where
        Self: TryCanReceive<T>,
    {
        <Self as TryCanReceive<T>>::try_give(self, item)
    }
}

/// Wrapper around `IntoIterator` to avoid trait impl conflicts.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iter<I>(pub I)
where
    I: IntoIterator;
impl<I> IntoIterator for Iter<I>
where
    I: IntoIterator,
{
    type Item = I::Item;
    type IntoIter = I::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
