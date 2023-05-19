//! Simple singleton or array collection with [`cc_traits`](crate::cc_traits) implementations.

use std::borrow::Borrow;
use std::hash::Hash;

use crate::cc_traits::{
    covariant_item_mut, covariant_item_ref, covariant_key_ref, Collection, CollectionMut,
    CollectionRef, Get, GetKeyValue, GetKeyValueMut, GetMut, Iter, IterMut, Keyed, KeyedRef, Len,
    MapIter, MapIterMut,
};

/// A [`Vec`]-wrapper representing a naively-implemented set.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VecSet<T>(pub Vec<T>);
impl<T> IntoIterator for VecSet<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

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
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
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
    type ItemMut<'a> = &'a mut Self::Item
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
    type Iter<'a> = std::slice::Iter<'a, T>
    where
        Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
}
impl<T> IterMut for VecSet<T> {
    type IterMut<'a> = std::slice::IterMut<'a, T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.0.iter_mut()
    }
}

/// A [`Vec`]-wrapper representing a naively implemented map.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    type IntoIter = std::iter::Zip<std::vec::IntoIter<K>, std::vec::IntoIter<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.vals)
    }
}
impl<K, V> Collection for VecMap<K, V> {
    type Item = V;
}
impl<K, V> Len for VecMap<K, V> {
    fn len(&self) -> usize {
        std::cmp::min(self.keys.len(), self.vals.len())
    }

    fn is_empty(&self) -> bool {
        self.keys.is_empty() || self.vals.is_empty()
    }
}
impl<K, V> CollectionRef for VecMap<K, V> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
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
    type ItemMut<'a> = &'a mut Self::Item
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
    type KeyRef<'a> = &'a Self::Key
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
impl<K, V> MapIter for VecMap<K, V> {
    type Iter<'a> = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.keys.iter().zip(self.vals.iter())
    }
}
impl<K, V> MapIterMut for VecMap<K, V> {
    type IterMut<'a> = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::IterMut<'a, V>>
	where
		Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.keys.iter().zip(self.vals.iter_mut())
    }
}

/// A wrapper around an item, representing a singleton set.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SingletonSet<T>(pub T);
impl<T> IntoIterator for SingletonSet<T> {
    type Item = T;
    type IntoIter = std::iter::Once<T>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0)
    }
}
impl<T> From<T> for SingletonSet<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}
impl<T> Collection for SingletonSet<T> {
    type Item = T;
}
impl<T> Len for SingletonSet<T> {
    fn len(&self) -> usize {
        1
    }
}
impl<T> CollectionRef for SingletonSet<T> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<'a, Q, T> Get<&'a Q> for SingletonSet<T>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        (key == self.0.borrow()).then_some(&self.0)
    }
}
impl<T> CollectionMut for SingletonSet<T> {
    type ItemMut<'a> = &'a mut T
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, T> GetMut<&'a Q> for SingletonSet<T>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        (key == self.0.borrow()).then_some(&mut self.0)
    }
}
impl<T> Iter for SingletonSet<T> {
    type Iter<'a> = std::iter::Once<&'a T>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        std::iter::once(&self.0)
    }
}
impl<T> IterMut for SingletonSet<T> {
    type IterMut<'a> = std::iter::Once<&'a mut T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        std::iter::once(&mut self.0)
    }
}

/// A key-value entry wrapper representing a singleton map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SingletonMap<K, V>(pub K, pub V);
impl<K, V> IntoIterator for SingletonMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::iter::Once<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once((self.0, self.1))
    }
}
impl<K, V> From<(K, V)> for SingletonMap<K, V> {
    fn from((k, v): (K, V)) -> Self {
        Self(k, v)
    }
}
impl<K, V> Collection for SingletonMap<K, V> {
    type Item = V;
}
impl<K, V> Len for SingletonMap<K, V> {
    fn len(&self) -> usize {
        1
    }
}
impl<K, V> CollectionRef for SingletonMap<K, V> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<'a, Q, K, V> Get<&'a Q> for SingletonMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        (key == self.0.borrow()).then_some(&self.1)
    }
}
impl<K, V> CollectionMut for SingletonMap<K, V> {
    type ItemMut<'a> = &'a mut Self::Item
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, K, V> GetMut<&'a Q> for SingletonMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        (key == self.0.borrow()).then_some(&mut self.1)
    }
}
impl<K, V> Keyed for SingletonMap<K, V> {
    type Key = K;
}
impl<K, V> KeyedRef for SingletonMap<K, V> {
    type KeyRef<'a> = &'a Self::Key
	where
		Self: 'a;

    covariant_key_ref!();
}
impl<'a, Q, K, V> GetKeyValue<&'a Q> for SingletonMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value(&self, key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemRef<'_>)> {
        (key == self.0.borrow()).then_some((&self.0, &self.1))
    }
}
impl<'a, Q, K, V> GetKeyValueMut<&'a Q> for SingletonMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value_mut(&mut self, key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemMut<'_>)> {
        (key == self.0.borrow()).then_some((&self.0, &mut self.1))
    }
}
impl<K, V> Iter for SingletonMap<K, V> {
    type Iter<'a> = std::iter::Once<&'a V>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        std::iter::once(&self.1)
    }
}
// impl<K, V> SimpleKeyedRef for SingletonMap<K, V> {
//     simple_keyed_ref!();
// }
impl<K, V> MapIter for SingletonMap<K, V> {
    type Iter<'a> = std::iter::Once<(&'a K, &'a V)>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        std::iter::once((&self.0, &self.1))
    }
}
impl<K, V> MapIterMut for SingletonMap<K, V> {
    type IterMut<'a> = std::iter::Once<(&'a K, &'a mut V)>
	where
		Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        std::iter::once((&self.0, &mut self.1))
    }
}

/// An array wrapper representing a fixed-size set (modulo duplicate items).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArraySet<T, const N: usize>(pub [T; N]);
impl<T, const N: usize> IntoIterator for ArraySet<T, N> {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<T, const N: usize> From<[T; N]> for ArraySet<T, N> {
    fn from(value: [T; N]) -> Self {
        Self(value)
    }
}
impl<T, const N: usize> Collection for ArraySet<T, N> {
    type Item = T;
}
impl<T, const N: usize> Len for ArraySet<T, N> {
    fn len(&self) -> usize {
        N
    }
}
impl<T, const N: usize> CollectionRef for ArraySet<T, N> {
    type ItemRef<'a> = &'a T
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<'a, Q, T, const N: usize> Get<&'a Q> for ArraySet<T, N>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.0
            .iter()
            .position(|item| key == item.borrow())
            .map(|i| &self.0[i])
    }
}
impl<T, const N: usize> CollectionMut for ArraySet<T, N> {
    type ItemMut<'a> = &'a mut T
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, T, const N: usize> GetMut<&'a Q> for ArraySet<T, N>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        self.0
            .iter()
            .position(|item| key == item.borrow())
            .map(|i| &mut self.0[i])
    }
}
impl<T, const N: usize> Iter for ArraySet<T, N> {
    type Iter<'a> = std::slice::Iter<'a, T>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
}

/// An array wrapper representing a fixed-size map.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayMap<K, V, const N: usize> {
    /// Keys, corresponding 1:1 with `vals`.
    pub keys: [K; N],
    /// Values, corresponding 1:1 with `keys`.
    pub vals: [V; N],
}
impl<K, V, const N: usize> IntoIterator for ArrayMap<K, V, N> {
    type Item = (K, V);
    type IntoIter = std::iter::Zip<std::array::IntoIter<K, N>, std::array::IntoIter<V, N>>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.into_iter().zip(self.vals)
    }
}
impl<K, V, const N: usize> From<[(K, V); N]> for ArrayMap<K, V, N> {
    fn from(value: [(K, V); N]) -> Self {
        let mut keys = Vec::with_capacity(N);
        let mut vals = Vec::with_capacity(N);
        for (k, v) in value {
            keys.push(k);
            vals.push(v);
        }
        Self {
            keys: keys.try_into().ok().unwrap(),
            vals: vals.try_into().ok().unwrap(),
        }
    }
}
impl<K, V, const N: usize> Collection for ArrayMap<K, V, N> {
    type Item = V;
}
impl<K, V, const N: usize> Len for ArrayMap<K, V, N> {
    fn len(&self) -> usize {
        N
    }
}
impl<K, V, const N: usize> CollectionRef for ArrayMap<K, V, N> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<'a, Q, K, V, const N: usize> Get<&'a Q> for ArrayMap<K, V, N>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.keys
            .iter()
            .position(|k| key == k.borrow())
            .map(|i| &self.vals[i])
    }
}
impl<K, V, const N: usize> CollectionMut for ArrayMap<K, V, N> {
    type ItemMut<'a> = &'a mut Self::Item
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, K, V, const N: usize> GetMut<&'a Q> for ArrayMap<K, V, N>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        self.keys
            .iter()
            .position(|item| key == item.borrow())
            .map(|i| &mut self.vals[i])
    }
}
impl<K, V, const N: usize> Keyed for ArrayMap<K, V, N> {
    type Key = K;
}
impl<K, V, const N: usize> KeyedRef for ArrayMap<K, V, N> {
    type KeyRef<'a> = &'a Self::Key
	where
		Self: 'a;

    covariant_key_ref!();
}
impl<'a, Q, K, V, const N: usize> GetKeyValue<&'a Q> for ArrayMap<K, V, N>
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
impl<'a, Q, K, V, const N: usize> GetKeyValueMut<&'a Q> for ArrayMap<K, V, N>
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
impl<K, V, const N: usize> Iter for ArrayMap<K, V, N> {
    type Iter<'a> = std::slice::Iter<'a, V>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.vals.iter()
    }
}
impl<K, V, const N: usize> MapIter for ArrayMap<K, V, N> {
    type Iter<'a> = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::Iter<'a, V>>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.keys.iter().zip(self.vals.iter())
    }
}
impl<K, V, const N: usize> MapIterMut for ArrayMap<K, V, N> {
    type IterMut<'a> = std::iter::Zip<std::slice::Iter<'a, K>, std::slice::IterMut<'a, V>>
	where
		Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.keys.iter().zip(self.vals.iter_mut())
    }
}

// /// A boolean-masked fixed-size array wrapper which implements `Collection`.
// #[derive(Clone, Copy, PartialEq, Eq, Hash)]
// pub struct MaskedArray<T, const N: usize> {
//     /// The boolean mask.
//     pub mask: [bool; N],
//     /// The collection items.
//     pub vals: [T; N],
// }
// impl<T, const N: usize> IntoIterator for MaskedArray<T, N> {
//     type Item = T;
//     type IntoIter = impl Iterator<Item = Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         self.mask
//             .into_iter()
//             .zip(self.vals)
//             .filter(|(mask, _)| *mask)
//             .map(|(_, val)| val)
//     }
// }
