pub mod handoff_list;
mod tee;
mod vector;

use std::any::Any;
use std::cell::RefMut;

pub use tee::TeeingHandoff;
pub use vector::VecHandoff;

pub trait TryCanReceive<T> {
    fn try_give(&self, item: T) -> Result<T, T>;
}
pub trait CanReceive<T> {
    fn give(&self, item: T) -> T;
}

/// A handle onto the metadata part of a [Handoff], with no element type.
pub trait HandoffMeta: Any {
    fn any_ref(&self) -> &dyn Any;

    // TODO(justin): more fine-grained info here.
    fn is_bottom(&self) -> bool;
}

pub trait Handoff: Default + HandoffMeta {
    type Inner;

    fn take_inner(&self) -> Self::Inner;

    fn borrow_mut_swap(&self) -> RefMut<Self::Inner>;

    fn give<T>(&self, item: T) -> T
    where
        Self: CanReceive<T>,
    {
        <Self as CanReceive<T>>::give(self, item)
    }

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
