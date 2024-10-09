//! Module containing the [`UnionFind`] lattice and aliases for different datastructures.

use std::cell::Cell;
use std::cmp::Ordering::{self, *};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

use crate::cc_traits::{Keyed, Map, MapIter, MapMut};
use crate::collections::{ArrayMap, OptionMap, SingletonMap, VecMap};
use crate::{Atomize, DeepReveal, IsBot, IsTop, LatticeFrom, LatticeOrd, Max, Merge, Min};

// TODO(mingwei): handling malformed trees - parents must be Ord smaller than children.

/// Union-find lattice.
///
/// Each value of `K` in the map represents an item in a set. When two lattices instances are
/// merged, any sets with common elements will be unioned together.
///
/// [`Self::union(a, b)`] unions two sets together, which is equivalent to merging in a
/// `UnionFindSingletonMap` atom of `(a, b)` (or `(b, a)`).
///
/// Any union-find consisting only of singleton sets is bottom.
///
/// ## Hasse diagram of partitions of a set of size four:
///
/// <a href="https://en.wikipedia.org/wiki/File:Set_partitions_4;_Hasse;_circles.svg">
///     <img src="https://upload.wikimedia.org/wikipedia/commons/3/32/Set_partitions_4%3B_Hasse%3B_circles.svg" width="500" />
/// </a>
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnionFind<Map>(Map);
impl<Map> UnionFind<Map> {
    /// Create a new `UnionFind` from a `Map`.
    pub fn new(val: Map) -> Self {
        Self(val)
    }

    /// Create a new `UnionFind` from an `Into<Map>`.
    pub fn new_from(val: impl Into<Map>) -> Self {
        Self::new(val.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> &Map {
        &self.0
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> &mut Map {
        &mut self.0
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> Map {
        self.0
    }
}

impl<Map> DeepReveal for UnionFind<Map> {
    type Revealed = Map;

    fn deep_reveal(self) -> Self::Revealed {
        self.0
    }
}

impl<Map, K> UnionFind<Map>
where
    Map: MapMut<K, Cell<K>, Key = K, Item = Cell<K>>,
    K: Copy + Eq,
{
    /// Union the sets containg `a` and `b`.
    ///
    /// Returns true if the sets changed, false if `a` and `b` were already in the same set. Once
    /// this returns false it will always return false for the same `a` and `b`, therefore it
    /// returns a `Min<bool>` lattice.
    pub fn union(&mut self, a: K, b: K) -> Min<bool> {
        let a_root = self.find(a);
        let b_root = self.find(b);
        if a_root == b_root {
            Min::new(false)
        } else {
            self.0.insert(b_root, Cell::new(a_root));
            Min::new(true)
        }
    }
}

impl<MapSelf, K> UnionFind<MapSelf>
where
    MapSelf: Map<K, Cell<K>, Key = K, Item = Cell<K>>,
    K: Copy + Eq,
{
    /// Returns if `a` and `b` are in the same set.
    ///
    /// This method is monotonic: once this returns true it will always return true for the same
    /// `a` and `b`, therefore it returns a `Max<bool>` lattice.
    pub fn same(&self, a: K, b: K) -> Max<bool> {
        Max::new(a == b || self.find(a) == self.find(b))
    }

    /// Finds the representative root node for `item`.
    fn find(&self, mut item: K) -> K {
        let mut root = item;
        while let Some(parent) = self.0.get(&root) {
            // If root is the representative.
            if parent.get() == root {
                break;
            }
            // Loop detected, close the end.
            if parent.get() == item {
                parent.set(root);
                break;
            }
            root = parent.get();
        }
        while item != root {
            item = self.0.get(&item).unwrap().replace(root);
        }
        item
    }
}

impl<MapSelf, MapOther, K> Merge<UnionFind<MapOther>> for UnionFind<MapSelf>
where
    MapSelf: MapMut<K, Cell<K>, Key = K, Item = Cell<K>>,
    MapOther: IntoIterator<Item = (K, Cell<K>)>,
    K: Copy + Eq,
{
    fn merge(&mut self, other: UnionFind<MapOther>) -> bool {
        let mut changed = false;
        for (item, parent) in other.0 {
            // Do not short circuit.
            changed |= self.union(item, parent.get()).into_reveal();
        }
        changed
    }
}

impl<MapSelf, MapOther, K> LatticeFrom<UnionFind<MapOther>> for UnionFind<MapSelf>
where
    MapSelf: Keyed<Key = K, Item = Cell<K>> + FromIterator<(K, Cell<K>)>,
    MapOther: IntoIterator<Item = (K, Cell<K>)>,
    K: Copy + Eq,
{
    fn lattice_from(other: UnionFind<MapOther>) -> Self {
        Self(other.0.into_iter().collect())
    }
}

impl<MapSelf, MapOther, K> PartialOrd<UnionFind<MapOther>> for UnionFind<MapSelf>
where
    MapSelf: MapMut<K, Cell<K>, Key = K, Item = Cell<K>> + MapIter,
    MapOther: MapMut<K, Cell<K>, Key = K, Item = Cell<K>> + MapIter,
    K: Copy + Eq,
{
    fn partial_cmp(&self, other: &UnionFind<MapOther>) -> Option<Ordering> {
        let self_any_greater = self
            .0
            .iter()
            .any(|(item, parent)| !other.same(*item, parent.get()).into_reveal());
        let other_any_greater = other
            .0
            .iter()
            .any(|(item, parent)| !self.same(*item, parent.get()).into_reveal());
        match (self_any_greater, other_any_greater) {
            (true, true) => None,
            (true, false) => Some(Greater),
            (false, true) => Some(Less),
            (false, false) => Some(Equal),
        }
    }
}
impl<MapSelf, MapOther> LatticeOrd<UnionFind<MapOther>> for UnionFind<MapSelf> where
    Self: PartialOrd<UnionFind<MapOther>>
{
}

impl<MapSelf, MapOther, K> PartialEq<UnionFind<MapOther>> for UnionFind<MapSelf>
where
    MapSelf: MapMut<K, Cell<K>, Key = K, Item = Cell<K>> + MapIter,
    MapOther: MapMut<K, Cell<K>, Key = K, Item = Cell<K>> + MapIter,
    K: Copy + Eq,
{
    fn eq(&self, other: &UnionFind<MapOther>) -> bool {
        !(self
            .0
            .iter()
            .any(|(item, parent)| !other.same(*item, parent.get()).into_reveal())
            || other
                .0
                .iter()
                .any(|(item, parent)| !self.same(*item, parent.get()).into_reveal()))
    }
}
impl<Map> Eq for UnionFind<Map> where Self: PartialEq {}

impl<Map, K> IsBot for UnionFind<Map>
where
    Map: MapIter<Key = K, Item = Cell<K>>,
    K: Copy + Eq,
{
    fn is_bot(&self) -> bool {
        self.0.iter().all(|(a, b)| *a == b.get())
    }
}

impl<Map> IsTop for UnionFind<Map> {
    fn is_top(&self) -> bool {
        false
    }
}

impl<Map, K> Atomize for UnionFind<Map>
where
    Map: 'static + MapMut<K, Cell<K>, Key = K, Item = Cell<K>> + IntoIterator<Item = (K, Cell<K>)>,
    K: 'static + Copy + Eq,
{
    type Atom = UnionFindSingletonMap<K>;

    // TODO: use impl trait, then remove 'static.
    type AtomIter = Box<dyn Iterator<Item = Self::Atom>>;

    fn atomize(self) -> Self::AtomIter {
        Box::new(
            self.0
                .into_iter()
                .filter(|(a, b)| *a != b.get())
                .map(UnionFindSingletonMap::new_from),
        )
    }
}

/// [`std::collections::HashMap`]-backed [`UnionFind`] lattice.
pub type UnionFindHashMap<K> = UnionFind<HashMap<K, Cell<K>>>;

/// [`std::collections::BTreeMap`]-backed [`UnionFind`] lattice.
pub type UnionFindBTreeMap<K> = UnionFind<BTreeMap<K, Cell<K>>>;

/// [`Vec`]-backed [`UnionFind`] lattice.
pub type UnionFindVec<K> = UnionFind<VecMap<K, Cell<K>>>;

/// Array-backed [`UnionFind`] lattice.
pub type UnionFindArrayMap<K, const N: usize> = UnionFind<ArrayMap<K, Cell<K>, N>>;

/// [`crate::collections::SingletonMap`]-backed [`UnionFind`] lattice.
pub type UnionFindSingletonMap<K> = UnionFind<SingletonMap<K, Cell<K>>>;

/// [`Option`]-backed [`UnionFind`] lattice.
pub type UnionFindOptionMap<K> = UnionFind<OptionMap<K, Cell<K>>>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::{check_all, check_atomize_each};

    #[test]
    fn test_basic() {
        let mut my_uf_a = <UnionFindHashMap<char>>::default();
        let my_uf_b = <UnionFindSingletonMap<char>>::new(SingletonMap('c', Cell::new('a')));
        let my_uf_c = UnionFindSingletonMap::new_from(('c', Cell::new('b')));

        assert!(!my_uf_a.same('c', 'a').into_reveal());
        assert!(!my_uf_a.same('c', 'b').into_reveal());
        assert!(!my_uf_a.same('a', 'b').into_reveal());
        assert!(!my_uf_a.same('a', 'z').into_reveal());
        assert_eq!('z', my_uf_a.find('z'));

        my_uf_a.merge(my_uf_b);

        assert!(my_uf_a.same('c', 'a').into_reveal());
        assert!(!my_uf_a.same('c', 'b').into_reveal());
        assert!(!my_uf_a.same('a', 'b').into_reveal());
        assert!(!my_uf_a.same('a', 'z').into_reveal());
        assert_eq!('z', my_uf_a.find('z'));

        my_uf_a.merge(my_uf_c);

        assert!(my_uf_a.same('c', 'a').into_reveal());
        assert!(my_uf_a.same('c', 'b').into_reveal());
        assert!(my_uf_a.same('a', 'b').into_reveal());
        assert!(!my_uf_a.same('a', 'z').into_reveal());
        assert_eq!('z', my_uf_a.find('z'));
    }

    // Make sure loops are considered as one group and don't hang.
    #[test]
    fn test_malformed() {
        {
            let my_uf = <UnionFindBTreeMap<char>>::new_from([
                ('a', Cell::new('b')),
                ('b', Cell::new('c')),
                ('c', Cell::new('a')),
            ]);
            println!("{:?}", my_uf);
            assert!(my_uf.same('a', 'b').into_reveal());
            println!("{:?}", my_uf);
        }
        {
            let my_uf = <UnionFindBTreeMap<char>>::new_from([
                ('a', Cell::new('b')),
                ('b', Cell::new('c')),
                ('c', Cell::new('d')),
                ('d', Cell::new('a')),
            ]);
            println!("{:?}", my_uf);
            assert!(my_uf.same('a', 'b').into_reveal());
            println!("{:?}", my_uf);
        }
    }

    #[test]
    fn consistency_atomize() {
        let items = &[
            <UnionFindHashMap<char>>::default(),
            <UnionFindHashMap<_>>::new_from([('a', Cell::new('a'))]),
            <UnionFindHashMap<_>>::new_from([('a', Cell::new('a')), ('b', Cell::new('a'))]),
            <UnionFindHashMap<_>>::new_from([('b', Cell::new('a'))]),
            <UnionFindHashMap<_>>::new_from([('b', Cell::new('a')), ('c', Cell::new('b'))]),
            <UnionFindHashMap<_>>::new_from([('b', Cell::new('a')), ('c', Cell::new('b'))]),
            <UnionFindHashMap<_>>::new_from([('d', Cell::new('b'))]),
            <UnionFindHashMap<_>>::new_from([
                ('b', Cell::new('a')),
                ('c', Cell::new('b')),
                ('d', Cell::new('a')),
            ]),
            <UnionFindHashMap<_>>::new_from([
                ('b', Cell::new('a')),
                ('c', Cell::new('b')),
                ('d', Cell::new('d')),
            ]),
        ];

        check_all(items);
        check_atomize_each(items);
    }
}
