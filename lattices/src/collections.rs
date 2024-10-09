//! Simple singleton or array collection with [`cc_traits`] implementations.

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;

use cc_traits::{
    covariant_item_mut, covariant_item_ref, covariant_key_ref, simple_collection_ref,
    simple_keyed_ref, Collection, CollectionMut, CollectionRef, Get, GetKeyValue, GetKeyValueMut,
    GetMut, Iter, IterMut, Keyed, KeyedRef, Len, MapIter, MapIterMut, SimpleCollectionRef,
    SimpleKeyedRef,
};

/// Trait for transforming the values of a map without changing the overall type of the data structure.
pub trait MapMapValues<OldVal> {
    /// Output type, should be `Self` but with `OldVal` replaced with `NewVal`.
    type MapValue<NewVal>;

    /// Map the values into using the `map_fn`.
    fn map_values<NewVal, MapFn>(self, map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal;
}
impl<Key, OldVal> MapMapValues<OldVal> for HashMap<Key, OldVal>
where
    Key: Eq + Hash,
{
    type MapValue<NewVal> = HashMap<Key, NewVal>;

    fn map_values<NewVal, MapFn>(self, mut map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal,
    {
        self.into_iter()
            .map(|(k, val)| (k, (map_fn)(val)))
            .collect()
    }
}
impl<Key, OldVal> MapMapValues<OldVal> for BTreeMap<Key, OldVal>
where
    Key: Eq + Ord,
{
    type MapValue<NewVal> = BTreeMap<Key, NewVal>;

    fn map_values<NewVal, MapFn>(self, mut map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal,
    {
        self.into_iter()
            .map(|(k, val)| (k, (map_fn)(val)))
            .collect()
    }
}

/// A [`Vec`]-wrapper representing a naively-implemented set.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
impl<K, V> SimpleKeyedRef for VecMap<K, V> {
    simple_keyed_ref!();
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

/// A type that will always be an empty set.
#[derive(Default, Debug, Clone, Copy, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmptySet<T> {
    _x: PhantomData<T>,
}

impl<T, Rhs> PartialEq<Rhs> for EmptySet<T>
where
    Rhs: Len,
{
    fn eq(&self, other: &Rhs) -> bool {
        other.is_empty()
    }
}

impl<T> Eq for EmptySet<T> {}

impl<T> Collection for EmptySet<T> {
    type Item = T;
}

impl<T> CollectionRef for EmptySet<T> {
    type ItemRef<'a> = &'a Self::Item where Self::Item: 'a;

    covariant_item_ref!();
}
impl<T> SimpleCollectionRef for EmptySet<T> {
    simple_collection_ref!();
}

impl<'a, Q, T> Get<&'a Q> for EmptySet<T> {
    fn get(&self, _key: &'a Q) -> Option<Self::ItemRef<'_>> {
        None
    }
}

impl<T> Len for EmptySet<T> {
    fn len(&self) -> usize {
        0
    }
}

impl<T> Iter for EmptySet<T> {
    type Iter<'a> = std::iter::Empty<&'a T> where T: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        std::iter::empty()
    }
}

impl<T> IntoIterator for EmptySet<T> {
    type Item = T;
    type IntoIter = std::iter::Empty<T>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::empty()
    }
}

/// A wrapper around an item, representing a singleton set.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
impl<T> SimpleCollectionRef for SingletonSet<T> {
    simple_collection_ref!();
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
#[derive(Debug, Clone, Copy, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EmptyMap<K, V>(pub PhantomData<K>, pub PhantomData<V>);
impl<K, V> IntoIterator for EmptyMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::iter::Empty<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::empty()
    }
}

impl<K, V, Rhs> PartialEq<Rhs> for EmptyMap<K, V>
where
    Rhs: Len,
{
    fn eq(&self, other: &Rhs) -> bool {
        other.is_empty()
    }
}

impl<K, V> Eq for EmptyMap<K, V> {}

impl<K, V> Collection for EmptyMap<K, V> {
    type Item = V;
}
impl<K, V> Len for EmptyMap<K, V> {
    fn len(&self) -> usize {
        0
    }
}
impl<K, V> CollectionRef for EmptyMap<K, V> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<K, V> SimpleCollectionRef for EmptyMap<K, V> {
    simple_collection_ref!();
}
impl<'a, Q, K, V> Get<&'a Q> for EmptyMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, _key: &'a Q) -> Option<Self::ItemRef<'_>> {
        None
    }
}
impl<K, V> CollectionMut for EmptyMap<K, V> {
    type ItemMut<'a> = &'a mut Self::Item
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, K, V> GetMut<&'a Q> for EmptyMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, _key: &'a Q) -> Option<Self::ItemMut<'_>> {
        None
    }
}
impl<K, V> Keyed for EmptyMap<K, V> {
    type Key = K;
}
impl<K, V> KeyedRef for EmptyMap<K, V> {
    type KeyRef<'a> = &'a Self::Key
	where
		Self: 'a;

    covariant_key_ref!();
}
impl<'a, Q, K, V> GetKeyValue<&'a Q> for EmptyMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value(&self, _key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemRef<'_>)> {
        None
    }
}
impl<'a, Q, K, V> GetKeyValueMut<&'a Q> for EmptyMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value_mut(&mut self, _key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemMut<'_>)> {
        None
    }
}
impl<K, V> Iter for EmptyMap<K, V> {
    type Iter<'a> = std::iter::Empty<&'a V>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        std::iter::empty()
    }
}
impl<K, V> SimpleKeyedRef for EmptyMap<K, V> {
    simple_keyed_ref!();
}
impl<K, V> MapIter for EmptyMap<K, V> {
    type Iter<'a> = std::iter::Empty<(&'a K, &'a V)>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        std::iter::empty()
    }
}
impl<K, V> MapIterMut for EmptyMap<K, V> {
    type IterMut<'a> = std::iter::Empty<(&'a K, &'a mut V)>
	where
		Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        std::iter::empty()
    }
}

/// A key-value entry wrapper representing a singleton map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
impl<K, V> SimpleCollectionRef for SingletonMap<K, V> {
    simple_collection_ref!();
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
impl<K, V> SimpleKeyedRef for SingletonMap<K, V> {
    simple_keyed_ref!();
}
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
impl<K, OldVal> MapMapValues<OldVal> for SingletonMap<K, OldVal> {
    type MapValue<NewVal> = SingletonMap<K, NewVal>;

    fn map_values<NewVal, MapFn>(self, mut map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal,
    {
        let Self(key, val) = self;
        let val = (map_fn)(val);
        SingletonMap(key, val)
    }
}

/// A wrapper around `Option`, representing either a singleton or empty set.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionSet<T>(pub Option<T>);
impl<T> Default for OptionSet<T> {
    fn default() -> Self {
        Self(None)
    }
}
impl<T> IntoIterator for OptionSet<T> {
    type Item = T;
    type IntoIter = std::option::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<T, U> From<U> for OptionSet<T>
where
    U: Into<Option<T>>,
{
    fn from(value: U) -> Self {
        Self(value.into())
    }
}
impl<T> Collection for OptionSet<T> {
    type Item = T;
}
impl<T> Len for OptionSet<T> {
    fn len(&self) -> usize {
        self.0.is_some() as usize
    }
}
impl<T> CollectionRef for OptionSet<T> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<T> SimpleCollectionRef for OptionSet<T> {
    simple_collection_ref!();
}
impl<'a, Q, T> Get<&'a Q> for OptionSet<T>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.0.as_ref().filter(|inner| key == (**inner).borrow())
    }
}
impl<T> CollectionMut for OptionSet<T> {
    type ItemMut<'a> = &'a mut T
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, T> GetMut<&'a Q> for OptionSet<T>
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        self.0.as_mut().filter(|inner| key == (**inner).borrow())
    }
}
impl<T> Iter for OptionSet<T> {
    type Iter<'a> = std::option::Iter<'a, T>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter()
    }
}
impl<T> IterMut for OptionSet<T> {
    type IterMut<'a> = std::option::IterMut<'a, T>
    where
        Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.0.iter_mut()
    }
}

/// A key-value entry wrapper around `Option<(K, V)>` representing a singleton or empty map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptionMap<K, V>(pub Option<(K, V)>);
impl<K, V> Default for OptionMap<K, V> {
    fn default() -> Self {
        Self(None)
    }
}
impl<K, V> IntoIterator for OptionMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::option::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<K, V, U> From<U> for OptionMap<K, V>
where
    U: Into<Option<(K, V)>>,
{
    fn from(kv: U) -> Self {
        Self(kv.into())
    }
}
impl<K, V> Collection for OptionMap<K, V> {
    type Item = V;
}
impl<K, V> Len for OptionMap<K, V> {
    fn len(&self) -> usize {
        self.0.is_some() as usize
    }
}
impl<K, V> CollectionRef for OptionMap<K, V> {
    type ItemRef<'a> = &'a Self::Item
    where
        Self: 'a;

    covariant_item_ref!();
}
impl<K, V> SimpleCollectionRef for OptionMap<K, V> {
    simple_collection_ref!();
}
impl<'a, Q, K, V> Get<&'a Q> for OptionMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get(&self, key: &'a Q) -> Option<Self::ItemRef<'_>> {
        self.0
            .as_ref()
            .filter(|(k, _v)| key == k.borrow())
            .map(|(_k, v)| v)
    }
}
impl<K, V> CollectionMut for OptionMap<K, V> {
    type ItemMut<'a> = &'a mut Self::Item
    where
        Self: 'a;

    covariant_item_mut!();
}
impl<'a, Q, K, V> GetMut<&'a Q> for OptionMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_mut(&mut self, key: &'a Q) -> Option<Self::ItemMut<'_>> {
        self.0
            .as_mut()
            .filter(|(k, _v)| key == k.borrow())
            .map(|(_k, v)| v)
    }
}
impl<K, V> Keyed for OptionMap<K, V> {
    type Key = K;
}
impl<K, V> KeyedRef for OptionMap<K, V> {
    type KeyRef<'a> = &'a Self::Key
	where
		Self: 'a;

    covariant_key_ref!();
}
impl<'a, Q, K, V> GetKeyValue<&'a Q> for OptionMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value(&self, key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemRef<'_>)> {
        self.0
            .as_ref()
            .filter(|(k, _v)| key == k.borrow())
            .map(|(k, v)| (k, v))
    }
}
impl<'a, Q, K, V> GetKeyValueMut<&'a Q> for OptionMap<K, V>
where
    K: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn get_key_value_mut(&mut self, key: &'a Q) -> Option<(Self::KeyRef<'_>, Self::ItemMut<'_>)> {
        self.0
            .as_mut()
            .filter(|(k, _v)| key == k.borrow())
            .map(|(k, v)| (&*k, v))
    }
}
impl<K, V> Iter for OptionMap<K, V> {
    type Iter<'a> = std::option::IntoIter<&'a V>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.as_ref().map(|(_k, v)| v).into_iter()
    }
}
impl<K, V> SimpleKeyedRef for OptionMap<K, V> {
    simple_keyed_ref!();
}
impl<K, V> MapIter for OptionMap<K, V> {
    type Iter<'a> = std::option::IntoIter<(&'a K, &'a V)>
	where
		Self: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        self.0.as_ref().map(|(k, v)| (k, v)).into_iter()
    }
}
impl<K, V> MapIterMut for OptionMap<K, V> {
    type IterMut<'a> = std::option::IntoIter<(&'a K, &'a mut V)>
	where
		Self: 'a;

    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        self.0.as_mut().map(|(k, v)| (&*k, v)).into_iter()
    }
}
impl<K, OldVal> MapMapValues<OldVal> for OptionMap<K, OldVal> {
    type MapValue<NewVal> = OptionMap<K, NewVal>;

    fn map_values<NewVal, MapFn>(self, mut map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal,
    {
        OptionMap(self.0.map(|(key, val)| (key, (map_fn)(val))))
    }
}

/// An array wrapper representing a fixed-size set (modulo duplicate items).
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
// TODO(mingwei): #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))] // https://stackoverflow.com/a/76695397/2398020
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
impl<T, const N: usize> SimpleCollectionRef for ArraySet<T, N> {
    simple_collection_ref!();
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
// TODO(mingwei): #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))] // https://stackoverflow.com/a/76695397/2398020
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
impl<K, V, const N: usize> SimpleCollectionRef for ArrayMap<K, V, N> {
    simple_collection_ref!();
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
impl<K, V, const N: usize> SimpleKeyedRef for ArrayMap<K, V, N> {
    simple_keyed_ref!();
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
impl<K, OldVal, const N: usize> MapMapValues<OldVal> for ArrayMap<K, OldVal, N> {
    type MapValue<NewVal> = ArrayMap<K, NewVal, N>;

    fn map_values<NewVal, MapFn>(self, map_fn: MapFn) -> Self::MapValue<NewVal>
    where
        MapFn: FnMut(OldVal) -> NewVal,
    {
        let Self { keys, vals } = self;
        let vals = vals.map(map_fn);
        ArrayMap { keys, vals }
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
