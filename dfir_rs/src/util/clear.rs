//! Module for the [`Clear`] trait.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::ffi::OsString;

/// A trait for abstracting over the `.clear()` method available in many
/// collection types.
pub trait Clear {
    /// Clears the collection without neccesarily freeing allocations.
    fn clear(&mut self);
}

/// A wrapper struct which implements [`Clear`] by setting self to `Default::default()`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClearDefault<T>(pub T)
where
    T: Default;
impl<T> Default for ClearDefault<T>
where
    T: Default,
{
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<T> Clear for ClearDefault<T>
where
    T: Default,
{
    fn clear(&mut self) {
        *self = ClearDefault(Default::default());
    }
}

macro_rules! clear_impl {
    (
        $t:ident $( < $( $g:ident ),* > )?
    ) => {
        impl $( < $( $g ),* > )? Clear for $t $( < $( $g ),* > )? {
            fn clear(&mut self) {
                self.clear()
            }
        }
    }
}

clear_impl!(BTreeMap<K, V>);
clear_impl!(BTreeSet<T>);
clear_impl!(BinaryHeap<T>);
clear_impl!(HashMap<K, V, S>);
clear_impl!(HashSet<T, S>);
clear_impl!(LinkedList<T>);
clear_impl!(OsString);
clear_impl!(String);
clear_impl!(Vec<T>);
clear_impl!(VecDeque<T>);

impl<T> Clear for Option<T> {
    fn clear(&mut self) {
        *self = None;
    }
}
