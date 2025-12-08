//! A Vec-based SlotMap-esque datastructure and corresponding Key type.

use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// A key into a SlotVec.
#[repr(transparent)]
pub struct Key<Tag: ?Sized> {
    index: usize,
    _phantom: PhantomData<Tag>,
}
impl<Tag: ?Sized> Key<Tag> {
    /// Creates a Key from a raw index. Avoid using this function directly.
    pub const fn from_raw(index: usize) -> Self {
        Key {
            index,
            _phantom: PhantomData,
        }
    }
}
impl<Tag: ?Sized> Clone for Key<Tag> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Tag: ?Sized> Copy for Key<Tag> {}
impl<Tag: ?Sized> PartialOrd for Key<Tag> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<Tag: ?Sized> Ord for Key<Tag> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}
impl<Tag: ?Sized> PartialEq for Key<Tag> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl<Tag: ?Sized> Eq for Key<Tag> {}
impl<Tag: ?Sized> Hash for Key<Tag> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
impl<Tag: ?Sized> Debug for Key<Tag> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Key({})", self.index)
    }
}
impl<Tag: ?Sized> Display for Key<Tag> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.index)
    }
}

/// A Vec-based SlotMap-esque datastructure without removes.
///
/// Analogous to [`slotmap::SlotMap`], but avoids the overhead of tracking removed keys.
#[repr(transparent)]
pub struct SlotVec<Tag: ?Sized, Val> {
    slots: Vec<Val>,
    _phantom: PhantomData<Tag>,
}
impl<Tag: ?Sized, Val> SlotVec<Tag, Val> {
    /// Creates a new `SlotVec`.
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Inserts a value into the `SlotVec` and returns the key.
    pub fn insert(&mut self, value: Val) -> Key<Tag> {
        let key = Key::from_raw(self.slots.len());
        self.slots.push(value);
        key
    }

    /// Use the provided function to generate a value given the key and insert it into the `SlotVec`.
    pub fn insert_with_key<F>(&mut self, func: F) -> Key<Tag>
    where
        F: FnOnce(Key<Tag>) -> Val,
    {
        let key = Key::from_raw(self.slots.len());
        self.slots.push((func)(key));
        key
    }

    /// Returns a reference to the value associated with the key.
    pub fn get(&self, key: Key<Tag>) -> Option<&Val> {
        self.slots.get(key.index)
    }

    /// Returns a mutable reference to the value associated with the key.
    pub fn get_mut(&mut self, key: Key<Tag>) -> Option<&mut Val> {
        self.slots.get_mut(key.index)
    }

    /// Returns the number of elements in the `SlotVec`.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns true if the `SlotVec` is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Iterate the key-value pairs, where the value is a shared reference.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (Key<Tag>, &'_ Val)> + ExactSizeIterator + FusedIterator + Clone
    {
        self.slots
            .iter()
            .enumerate()
            .map(|(idx, val)| (Key::from_raw(idx), val))
    }

    /// Iterate the key-value pairs, where the value is a exclusive reference.
    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (Key<Tag>, &'_ mut Val)> + ExactSizeIterator + FusedIterator
    {
        self.slots
            .iter_mut()
            .enumerate()
            .map(|(idx, val)| (Key::from_raw(idx), val))
    }

    /// Iterate over the values by shared reference.
    pub fn values(&self) -> std::slice::Iter<'_, Val> {
        self.slots.iter()
    }

    /// Iterate over the values by exclusive reference.
    pub fn values_mut(&mut self) -> std::slice::IterMut<'_, Val> {
        self.slots.iter_mut()
    }

    /// Iterate over the keys.
    pub fn keys(
        &self,
    ) -> impl '_ + DoubleEndedIterator<Item = Key<Tag>> + ExactSizeIterator + FusedIterator + Clone
    {
        self.iter().map(|(key, _)| key)
    }
}
impl<Tag: ?Sized, Val> Index<Key<Tag>> for SlotVec<Tag, Val> {
    type Output = Val;

    fn index(&self, key: Key<Tag>) -> &Self::Output {
        self.get(key).unwrap()
    }
}
impl<Tag: ?Sized, Val> IndexMut<Key<Tag>> for SlotVec<Tag, Val> {
    fn index_mut(&mut self, key: Key<Tag>) -> &mut Self::Output {
        self.get_mut(key).unwrap()
    }
}
impl<Key: ?Sized, Val> Default for SlotVec<Key, Val> {
    fn default() -> Self {
        Self::new()
    }
}

/// A secondary map used to associated data with keys from elements in an existing [`SlotVec`].
///
/// Analogous to [`slotmap::SecondaryMap`].
pub struct SecondarySlotVec<Tag: ?Sized, Val> {
    slots: Vec<Option<Val>>,
    _phantom: PhantomData<Tag>,
}
impl<Tag: ?Sized, Val> SecondarySlotVec<Tag, Val> {
    /// Creates a new `SecondarySlotVec`.
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Inserts a value into the `SecondarySlotVec` and returns the previous value associated with the key.
    pub fn insert(&mut self, key: Key<Tag>, value: Val) -> Option<Val> {
        if key.index >= self.slots.len() {
            self.slots.resize_with(key.index + 1, || None);
        }
        self.slots[key.index].replace(value)
    }

    /// Removes a value associated with the key from the `SecondarySlotVec` and returns it.
    pub fn remove(&mut self, key: Key<Tag>) -> Option<Val> {
        // TODO(mingwei): Shrink the vector?
        self.slots[key.index].take()
    }

    /// Returns a reference to the value associated with the key.
    pub fn get(&self, key: Key<Tag>) -> Option<&Val> {
        self.slots.get(key.index).and_then(|v| v.as_ref())
    }

    /// Returns a mutable reference to the value associated with the key.
    pub fn get_mut(&mut self, key: Key<Tag>) -> Option<&mut Val> {
        self.slots.get_mut(key.index).and_then(|v| v.as_mut())
    }

    /// Returns a mutable reference to the value associated with the key, inserting a default value
    /// if it doesn't yet exist.
    pub fn get_or_insert_with(&mut self, key: Key<Tag>, default: impl FnOnce() -> Val) -> &mut Val {
        if key.index >= self.slots.len() {
            self.slots.resize_with(key.index + 1, || None);
        }
        self.slots[key.index].get_or_insert_with(default)
    }

    /// Iterate the key-value pairs, where the value is a shared reference.
    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (Key<Tag>, &'_ Val)> + FusedIterator + Clone {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(idx, opt_val)| Some((Key::from_raw(idx), opt_val.as_ref()?)))
    }

    /// Iterate the key-value pairs, where the value is a exclusive reference.
    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (Key<Tag>, &'_ mut Val)> + FusedIterator {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(idx, opt_val)| Some((Key::from_raw(idx), opt_val.as_mut()?)))
    }

    /// Iterate over the values by shared reference.
    pub fn values(&self) -> impl DoubleEndedIterator<Item = &'_ Val> + FusedIterator + Clone {
        self.slots.iter().filter_map(Option::as_ref)
    }

    /// Iterate over the values by exclusive reference.
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &'_ mut Val> + FusedIterator {
        self.slots.iter_mut().filter_map(Option::as_mut)
    }

    /// Iterate over the keys.
    pub fn keys(&self) -> impl '_ + DoubleEndedIterator<Item = Key<Tag>> + FusedIterator + Clone {
        self.iter().map(|(key, _)| key)
    }
}
impl<Tag: ?Sized, Val> Default for SecondarySlotVec<Tag, Val> {
    fn default() -> Self {
        Self::new()
    }
}
impl<Tag: ?Sized, Val> Clone for SecondarySlotVec<Tag, Val>
where
    Val: Clone,
{
    fn clone(&self) -> Self {
        Self {
            slots: self.slots.clone(),
            _phantom: PhantomData,
        }
    }
}
impl<Tag: ?Sized, Val> Index<Key<Tag>> for SecondarySlotVec<Tag, Val> {
    type Output = Val;

    fn index(&self, key: Key<Tag>) -> &Self::Output {
        self.get(key).unwrap()
    }
}
impl<Tag: ?Sized, Val> IndexMut<Key<Tag>> for SecondarySlotVec<Tag, Val> {
    fn index_mut(&mut self, key: Key<Tag>) -> &mut Self::Output {
        self.get_mut(key).unwrap()
    }
}
impl<Tag: ?Sized, Val> FromIterator<(Key<Tag>, Val)> for SecondarySlotVec<Tag, Val> {
    fn from_iter<T: IntoIterator<Item = (Key<Tag>, Val)>>(iter: T) -> Self {
        let mut map = SecondarySlotVec::new();
        for (key, val) in iter {
            map.insert(key, val);
        }
        map
    }
}
