use alloc::vec::Vec;
use core::borrow::Borrow;
use core::hash::Hash;

use cc_traits::{
    Collection, CollectionMut, CollectionRef, Get, GetKeyValue, GetKeyValueMut, GetMut, Iter,
    IterMut, Keyed, KeyedRef, Len, MapIter, MapIterMut, SimpleCollectionRef, SimpleKeyedRef,
    covariant_item_mut, covariant_item_ref, covariant_key_ref, simple_collection_ref,
    simple_keyed_ref,
};

use super::MapMapValues;

/// A [`Vec`](Vec)-wrapper representing a naively-implemented set.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VecSet<T>(pub Vec<T>);
impl<T> IntoIterator for VecSet<T> {
    type Item = T;
    type IntoIter = alloc::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<T> From<Vec<T>> for VecSet<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}
impl<T> Collection for VecSet<T> {
    type Item = T;
}
impl<T> Len for VecSet<T> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
impl<T> CollectionRef for VecSet<T> {
    type ItemRef<'a>
        = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<T> SimpleCollectionRef for VecSet<T> {
    simple_collection_ref!();
}
impl<'a, Q, T> Get<&'a Q> for VecSet<T>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.0.iter().find(|&k| key == k.borrow())
    }
}
impl<T> CollectionMut for VecSet<T> {
    type ItemMut<'a>
        = &'a mut Self::Item
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, T> GetMut<&'a Q> for VecSet<T>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        self.0.iter_mut().find(|k| key == T::borrow(k))
    }
}
impl<T> Iter for VecSet<T> {
    type Iter<'a>
        = core::slice::Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
}
impl<T> IterMut for VecSet<T> {
    type IterMut<'a>
        = core::slice::IterMut<'a, T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.0.iter_mut()
    }
}

/// A [`Vec`]-wrapper representing a naively implemented map.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VecMap<K, V> {
    /// Keys, should be the same length as and correspond 1:1 to `vals`.
    pub keys: Vec<K>,
    /// Vals, should be the same length as and correspond 1:1 to `keys`.
    pub vals: Vec<V>,
}
impl<K, V> VecMap<K, V> {
    /// Create a new `VecMap` from the separate `keys` and `vals` vecs.
    ///
    /// Panics if `keys` and `vals` are not the same length.
    pub fn new(keys: Vec<K>, vals: Vec<V>) -> Self {
        assert_eq!(keys.len(), vals.len());
        Self { keys, vals }
    }
}
impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = core::iter::Zip<alloc::vec::IntoIter<K>, alloc::vec::IntoIter<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.vals)
    }
}
impl<K, V> Collection for VecMap<K, V> {
    type Item = V;
}
impl<K, V> Len for VecMap<K, V> {
    fn len(&self) -> usize {
        core::cmp::min(self.keys.len(), self.vals.len())
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty() || self.vals.is_empty()
    }
}
impl<K, V> CollectionRef for VecMap<K, V> {
    type ItemRef<'a>
        = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<K, V> SimpleCollectionRef for VecMap<K, V> {
    simple_collection_ref!();
}
impl<'a, Q, K, V> Get<&'a Q> for VecMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.keys
            .iter()
            .position(|k| key == k.borrow())
            .and_then(|i| self.vals.get(i))
    }
}
impl<K, V> CollectionMut for VecMap<K, V> {
    type ItemMut<'a>
        = &'a mut Self::Item
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, K, V> GetMut<&'a Q> for VecMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        self.keys
            .iter()
            .position(|k| key == k.borrow())
            .and_then(|i| self.vals.get_mut(i))
    }
}
impl<K, V> Keyed for VecMap<K, V> {
    type Key = K;
}
impl<K, V> KeyedRef for VecMap<K, V> {
    type KeyRef<'a>
        = &'a Self::Key
    where
        Self: 'a;

    covariant_key_ref!();
}
impl<'a, Q, K, V> GetKeyValue<&'a Q> for VecMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value(&self, key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemRef<'_>)> {
        self.keys
            .iter()
            .zip(self.vals.iter())
            .find(|(k, _v)| key == K::borrow(k))
    }
}
impl<'a, Q, K, V> GetKeyValueMut<&'a Q> for VecMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value_mut(&mut self, key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemMut<'_>)> {
        self.keys
            .iter()
            .zip(self.vals.iter_mut())
            .find(|(k, _v)| key == K::borrow(k))
    }
}
impl<K, V> SimpleKeyedRef for VecMap<K, V> {
    simple_keyed_ref!();
}
impl<K, V> MapIter for VecMap<K, V> {
    type Iter<'a>
        = core::iter::Zip<core::slice::Iter<'a, K>, core::slice::Iter<'a, V>>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.keys.iter().zip(self.vals.iter())
    }
}
impl<K, V> MapIterMut for VecMap<K, V> {
    type IterMut<'a>
        = core::iter::Zip<core::slice::Iter<'a, K>, core::slice::IterMut<'a, V>>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.keys.iter().zip(self.vals.iter_mut())
    }
}
impl<K, OldVal> MapMapValues<OldVal> for VecMap<K, OldVal> {
    type MapValue<NewVal> = VecMap<K, NewVal>;

    fn map_values<NewVal, MapFn>(self, map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal,
    {
        let Self { keys, vals } = self;
        let vals = vals.into_iter().map(map_fn).collect();
        VecMap { keys, vals }
    }
}
