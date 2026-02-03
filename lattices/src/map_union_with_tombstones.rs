//! Module containing the [`MapUnionWithTombstones`] lattice and aliases for different datastructures.
//!
//! See [`crate::tombstone`] for documentation on choosing a tombstone implementation.

use std::cmp::Ordering::{self, *};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use cc_traits::{Get, Iter, Len, Remove};

use crate::cc_traits::{GetMut, Keyed, Map, MapIter, SimpleKeyedRef};
use crate::collections::{EmptyMap, EmptySet, SingletonMap, SingletonSet};
use crate::tombstone::{FstTombstoneSet, RoaringTombstoneSet, TombstoneSet};
use crate::{IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

/// Map-union-with-tombstones compound lattice.
///
/// When a key is deleted from the map-union-with-tombstones lattice, it is removed from the underlying `map` and placed into
/// the `tombstones` set.
///
/// This forms the first invariant for this data structure. A key should appear either nowhere, in `map` or in `tombstones`.
/// but never in `map` and `tombstones` at the same time.
///
/// merging is done by merging the underlying `map` and then merging the `tombstones` set, then doing `map` = `map` - `tombstones`.
///
/// The implementation of `tombstones` can be any set-like thing. This allows a user to plug in their own set-like implementation.
/// For example, if the user knows that keys will be created and deleted strictly sequentially, then they could create a highly optimized set implementation
/// which would just be a single integer, correpsonding to the current key value that the set is up to. Queries for keys below that integer would return true,
/// queries for keys above it would return false.
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MapUnionWithTombstones<Map, TombstoneSet> {
    map: Map,
    tombstones: TombstoneSet,
}

impl<Map, TombstoneSet> MapUnionWithTombstones<Map, TombstoneSet> {
    /// Create a new `MapUnionWithTombstones` from a `Map` and a `TombstoneSet`.
    pub fn new(map: Map, tombstones: TombstoneSet) -> Self {
        Self { map, tombstones }
    }

    /// Create a new `MapUnionWithTombstones` from an `Into<Map>` and an `Into<TombstoneSet>`.
    pub fn new_from(map: impl Into<Map>, tombstones: impl Into<TombstoneSet>) -> Self {
        Self::new(map.into(), tombstones.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> (&Map, &TombstoneSet) {
        (&self.map, &self.tombstones)
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> (&mut Map, &mut TombstoneSet) {
        (&mut self.map, &mut self.tombstones)
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> (Map, TombstoneSet) {
        (self.map, self.tombstones)
    }
}

// Merge implementation using TombstoneSet trait for optimized union operations
impl<MapSelf, MapOther, K, ValSelf, ValOther, TombstoneSetSelf, TombstoneSetOther>
    Merge<MapUnionWithTombstones<MapOther, TombstoneSetOther>>
    for MapUnionWithTombstones<MapSelf, TombstoneSetSelf>
where
    MapSelf: Keyed<Key = K, Item = ValSelf>
        + Extend<(K, ValSelf)>
        + for<'a> GetMut<&'a K, Item = ValSelf>
        + for<'b> Remove<&'b K>,
    MapOther: IntoIterator<Item = (K, ValOther)>,
    ValSelf: Merge<ValOther> + LatticeFrom<ValOther>,
    ValOther: IsBot,
    TombstoneSetSelf: TombstoneSet<K>,
    TombstoneSetOther: IntoIterator<Item = K>,
{
    fn merge(&mut self, other: MapUnionWithTombstones<MapOther, TombstoneSetOther>) -> bool {
        let mut changed = false;

        // Collect other.tombstones into a vec to avoid borrowing issues
        let other_tombstones: Vec<_> = other.tombstones.into_iter().collect();
        let old_tombstones_len = self.tombstones.len();

        // This vec collect is needed to prevent simultaneous mut references `self.0.extend` and
        // `self.0.get_mut`.
        // TODO(mingwei): This could be fixed with a different structure, maybe some sort of
        // `Collection` entry API.
        let iter: Vec<_> = other
            .map
            .into_iter()
            .filter(|(k_other, val_other)| {
                !val_other.is_bot() && !self.tombstones.contains(k_other)
            })
            .filter_map(|(k_other, val_other)| {
                match self.map.get_mut(&k_other) {
                    // Key collision, merge into `self`.
                    Some(mut val_self) => {
                        changed |= val_self.merge(val_other);
                        None
                    }
                    // New value, convert for extending.
                    None => {
                        changed = true;
                        Some((k_other, ValSelf::lattice_from(val_other)))
                    }
                }
            })
            .collect();
        self.map.extend(iter);

        // Extend tombstones and remove tombstoned keys from the map
        self.tombstones
            .extend(other_tombstones.into_iter().inspect(|k| {
                self.map.remove(k);
            }));

        if old_tombstones_len != self.tombstones.len() {
            changed = true;
        }

        changed
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther, TombstoneSetSelf, TombstoneSetOther>
    LatticeFrom<MapUnionWithTombstones<MapOther, TombstoneSetOther>>
    for MapUnionWithTombstones<MapSelf, TombstoneSetSelf>
where
    MapSelf: Keyed<Key = K, Item = ValSelf> + FromIterator<(K, ValSelf)>,
    MapOther: IntoIterator<Item = (K, ValOther)>,
    ValSelf: LatticeFrom<ValOther>,
    TombstoneSetSelf: FromIterator<K>,
    TombstoneSetOther: IntoIterator<Item = K>,
{
    fn lattice_from(other: MapUnionWithTombstones<MapOther, TombstoneSetOther>) -> Self {
        Self {
            map: other
                .map
                .into_iter()
                .map(|(k_other, val_other)| (k_other, LatticeFrom::lattice_from(val_other)))
                .collect(),
            tombstones: other.tombstones.into_iter().collect(),
        }
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther, TombstoneSetSelf, TombstoneSetOther>
    PartialOrd<MapUnionWithTombstones<MapOther, TombstoneSetOther>>
    for MapUnionWithTombstones<MapSelf, TombstoneSetSelf>
where
    MapSelf: Map<K, ValSelf, Key = K, Item = ValSelf> + MapIter + SimpleKeyedRef,
    MapOther: Map<K, ValOther, Key = K, Item = ValOther> + MapIter + SimpleKeyedRef,
    ValSelf: PartialOrd<ValOther> + IsBot,
    ValOther: IsBot,
    TombstoneSetSelf: Len + Iter<Item = K> + for<'a> Get<&'a K>,
    TombstoneSetOther: Len + Iter<Item = K> + for<'a> Get<&'a K>,
{
    fn partial_cmp(
        &self,
        other: &MapUnionWithTombstones<MapOther, TombstoneSetOther>,
    ) -> Option<Ordering> {
        let self_tombstones_greater = self
            .tombstones
            .iter()
            .any(|k| !other.tombstones.contains(&*k));

        let other_tombstones_greater = other
            .tombstones
            .iter()
            .any(|k| !self.tombstones.contains(&*k));

        if self_tombstones_greater && other_tombstones_greater {
            return None;
        }

        let mut self_any_greater = false;
        let mut other_any_greater = false;
        let self_keys = self
            .map
            .iter()
            .filter(|(k, v)| {
                !v.is_bot() && !self.tombstones.contains(k) && !other.tombstones.contains(k)
            })
            .map(|(k, _v)| <MapSelf as SimpleKeyedRef>::into_ref(k));
        let other_keys = other
            .map
            .iter()
            .filter(|(k, v)| {
                !v.is_bot() && !self.tombstones.contains(k) && !other.tombstones.contains(k)
            })
            .map(|(k, _v)| <MapOther as SimpleKeyedRef>::into_ref(k));

        for k in self_keys.chain(other_keys) {
            match (self.map.get(k), other.map.get(k)) {
                (Some(self_value), Some(other_value)) => {
                    match self_value.partial_cmp(&*other_value)? {
                        Less => {
                            other_any_greater = true;
                        }
                        Greater => {
                            self_any_greater = true;
                        }
                        Equal => {}
                    }
                }
                (Some(_), None) => {
                    self_any_greater = true;
                }
                (None, Some(_)) => {
                    other_any_greater = true;
                }
                (None, None) => unreachable!(),
            }
            if self_any_greater && other_any_greater {
                return None;
            }
        }
        match (
            self_any_greater,
            other_any_greater,
            self_tombstones_greater,
            other_tombstones_greater,
        ) {
            (false, false, false, false) => Some(Equal),

            (false, false, true, false) => Some(Greater),
            (false, false, false, true) => Some(Less),

            (true, false, false, false) => Some(Greater),
            (false, true, false, false) => Some(Less),

            (true, false, true, false) => Some(Greater),
            (false, true, false, true) => Some(Less),

            (true, false, false, true) => None,
            (false, true, true, false) => None,

            (true, true, _, _) => unreachable!(),
            (_, _, true, true) => unreachable!(),
        }
    }
}
impl<MapSelf, MapOther, TombstoneSetSelf, TombstoneSetOther>
    LatticeOrd<MapUnionWithTombstones<MapOther, TombstoneSetOther>>
    for MapUnionWithTombstones<MapSelf, TombstoneSetSelf>
where
    Self: PartialOrd<MapUnionWithTombstones<MapOther, TombstoneSetOther>>,
{
}

impl<MapSelf, MapOther, K, ValSelf, ValOther, TombstoneSetSelf, TombstoneSetOther>
    PartialEq<MapUnionWithTombstones<MapOther, TombstoneSetOther>>
    for MapUnionWithTombstones<MapSelf, TombstoneSetSelf>
where
    MapSelf: Map<K, ValSelf, Key = K, Item = ValSelf> + MapIter + SimpleKeyedRef,
    MapOther: Map<K, ValOther, Key = K, Item = ValOther> + MapIter + SimpleKeyedRef,
    ValSelf: PartialEq<ValOther> + IsBot,
    ValOther: IsBot,
    TombstoneSetSelf: Len + Iter<Item = K> + for<'a> Get<&'a K>,
    TombstoneSetOther: Len + Iter<Item = K> + for<'b> Get<&'b K>,
{
    fn eq(&self, other: &MapUnionWithTombstones<MapOther, TombstoneSetOther>) -> bool {
        if self.tombstones.len() != other.tombstones.len() {
            return false;
        }

        if self
            .tombstones
            .iter()
            .any(|k| !other.tombstones.contains(&*k))
        {
            return false;
        }

        if other
            .tombstones
            .iter()
            .any(|k| !self.tombstones.contains(&*k))
        {
            return false;
        }

        let self_keys = self
            .map
            .iter()
            .filter(|(_k, v)| !v.is_bot())
            .map(|(k, _v)| <MapSelf as SimpleKeyedRef>::into_ref(k));
        let other_keys = other
            .map
            .iter()
            .filter(|(_k, v)| !v.is_bot())
            .map(|(k, _v)| <MapOther as SimpleKeyedRef>::into_ref(k));
        for k in self_keys.chain(other_keys) {
            match (self.map.get(k), other.map.get(k)) {
                (Some(self_value), Some(other_value)) => {
                    if *self_value != *other_value {
                        return false;
                    }
                }
                (None, None) => unreachable!(),
                _ => {
                    return false;
                }
            }
        }

        true
    }
}
impl<MapSelf, TombstoneSetSelf> Eq for MapUnionWithTombstones<MapSelf, TombstoneSetSelf> where
    Self: PartialEq
{
}

impl<Map, TombstoneSet> IsBot for MapUnionWithTombstones<Map, TombstoneSet>
where
    Map: Iter,
    Map::Item: IsBot,
    TombstoneSet: Len,
{
    fn is_bot(&self) -> bool {
        self.map.iter().all(|v| v.is_bot()) && self.tombstones.is_empty()
    }
}

impl<Map, TombstoneSet> IsTop for MapUnionWithTombstones<Map, TombstoneSet> {
    fn is_top(&self) -> bool {
        false
    }
}

/// [`std::collections::HashMap`]-backed [`MapUnionWithTombstones`] lattice.
pub type MapUnionHashMapWithTombstoneHashSet<K, Val> =
    MapUnionWithTombstones<HashMap<K, Val>, HashSet<K>>;

/// [`crate::collections::SingletonMap`]-backed [`MapUnionWithTombstones`] lattice.
pub type MapUnionWithTombstonesSingletonMapOnly<K, Val> =
    MapUnionWithTombstones<SingletonMap<K, Val>, EmptySet<K>>;

/// [`crate::collections::SingletonSet`]-backed [`MapUnionWithTombstones`] lattice.
pub type MapUnionWithTombstonesTombstoneSingletonSetOnly<K, Val> =
    MapUnionWithTombstones<EmptyMap<K, Val>, SingletonSet<K>>;

/// [`crate::tombstone::RoaringTombstoneSet`]-backed tombstone set with [`HashMap`] for the main map.
/// Provides space-efficient tombstone storage for u64 integer keys.
pub type MapUnionWithTombstonesRoaring<Val> =
    MapUnionWithTombstones<HashMap<u64, Val>, RoaringTombstoneSet>;

/// FST-backed tombstone set with [`HashMap`] for the main map.
/// Provides space-efficient, collision-free tombstone storage for String keys.
pub type MapUnionWithTombstonesFstString<Val> =
    MapUnionWithTombstones<HashMap<String, Val>, FstTombstoneSet<String>>;

#[cfg(test)]
mod test {
    use super::*;
    use crate::NaiveLatticeOrd;
    use crate::set_union::{SetUnion, SetUnionHashSet, SetUnionSingletonSet};
    use crate::test::check_all;

    #[test]
    fn test_map_union() {
        type K = &'static str;
        type V = usize;

        type M = MapUnionWithTombstones<HashMap<K, SetUnionHashSet<V>>, HashSet<K>>;
        type S = MapUnionWithTombstones<SingletonMap<K, SetUnionSingletonSet<V>>, EmptySet<K>>;
        type T = MapUnionWithTombstones<EmptyMap<K, SetUnion<EmptySet<V>>>, SingletonSet<K>>;

        let mut my_map_a = M::default();
        let my_map_b = S::new(
            SingletonMap("hello", SetUnion::new(SingletonSet(100))),
            Default::default(),
        );

        let my_map_c = T::new(Default::default(), SingletonSet("hello"));

        my_map_a.merge(my_map_b);
        my_map_a.merge(my_map_c);

        assert!(!my_map_a.as_reveal_ref().0.contains_key("hello"));
    }

    #[test]
    fn contrain1() {
        type T = MapUnionWithTombstones<HashMap<i32, SetUnion<HashSet<i32>>>, HashSet<i32>>;

        let a = T::new_from([], HashSet::from_iter([0]));
        let b = T::new_from(
            [(0, SetUnionHashSet::new_from([0]))],
            HashSet::from_iter([]),
        );

        assert_eq!(a.naive_cmp(&b), Some(Greater));
        assert_eq!(a.partial_cmp(&b), Some(Greater));

        let a = T::new_from([], HashSet::from_iter([1]));
        let b = T::new_from([(0, SetUnionHashSet::new_from([0]))], HashSet::default());

        assert_eq!(a.naive_cmp(&b), None);
        assert_eq!(a.partial_cmp(&b), None);
    }

    #[test]
    fn consistency() {
        type K = &'static str;
        type V = SetUnion<HashSet<i32>>;

        type M = MapUnionWithTombstones<HashMap<K, V>, HashSet<K>>;

        let mut test_vec = Vec::new();

        #[rustfmt::skip]
        {
            test_vec.push(M::new_from([], HashSet::from_iter([])));

            test_vec.push(M::new_from([], HashSet::from_iter(["a"])));
            test_vec.push(M::new_from([], HashSet::from_iter(["b"])));
            test_vec.push(M::new_from([], HashSet::from_iter(["a", "b"])));

            test_vec.push(M::new_from([("a", SetUnionHashSet::new_from([]))], HashSet::from_iter([])));
            test_vec.push(M::new_from([("a", SetUnionHashSet::new_from([0]))], HashSet::from_iter([])));
            test_vec.push(M::new_from([("a", SetUnionHashSet::new_from([1]))], HashSet::from_iter([])));
            test_vec.push(M::new_from([("a", SetUnionHashSet::new_from([0, 1]))], HashSet::from_iter([])));

            test_vec.push(M::new_from([("b", SetUnionHashSet::new_from([]))], HashSet::from_iter([])));
            test_vec.push(M::new_from([("b", SetUnionHashSet::new_from([0]))], HashSet::from_iter([])));
            test_vec.push(M::new_from([("b", SetUnionHashSet::new_from([1]))], HashSet::from_iter([])));
            test_vec.push(M::new_from([("b", SetUnionHashSet::new_from([0, 1]))], HashSet::from_iter([])));
        };

        check_all(&test_vec);
    }

    /// Check that a key with a value of bottom is the same as an empty map, etc.
    #[test]
    fn test_collapses_bot() {
        type K = &'static str;
        type V = SetUnion<HashSet<i32>>;

        type A = MapUnionWithTombstones<HashMap<K, V>, HashSet<K>>;
        type B = MapUnionWithTombstones<SingletonMap<K, V>, HashSet<K>>;

        let map_empty = A::default();

        let map_a_bot = B::new(SingletonMap("a", Default::default()), Default::default());
        let map_b_bot = B::new(SingletonMap("b", Default::default()), Default::default());

        assert_eq!(map_empty, map_a_bot);
        assert_eq!(map_empty, map_b_bot);
        assert_eq!(map_a_bot, map_b_bot);
    }

    #[test]
    fn roaring_u64_basic() {
        let mut x = MapUnionWithTombstonesRoaring::new_from(
            HashMap::from([
                (1u64, SetUnionHashSet::new_from([10])),
                (2, SetUnionHashSet::new_from([20])),
            ]),
            RoaringTombstoneSet::new(),
        );
        let mut y = MapUnionWithTombstonesRoaring::new_from(
            HashMap::from([
                (2u64, SetUnionHashSet::new_from([21])),
                (3, SetUnionHashSet::new_from([30])),
            ]),
            RoaringTombstoneSet::new(),
        );

        // Add tombstone for key 2
        y.as_reveal_mut().1.insert(2);

        x.merge(y);

        // Should have keys 1 and 3, but not 2 (tombstoned)
        assert!(!x.as_reveal_ref().0.contains_key(&2));
        assert!(x.as_reveal_ref().0.contains_key(&1));
        assert!(x.as_reveal_ref().0.contains_key(&3));
        assert!(x.as_reveal_ref().1.contains(&2));
    }

    #[test]
    fn fst_string_basic() {
        let mut x = MapUnionWithTombstonesFstString::new_from(
            HashMap::from([
                ("apple".to_owned(), SetUnionHashSet::new_from([1])),
                ("banana".to_owned(), SetUnionHashSet::new_from([2])),
            ]),
            FstTombstoneSet::new(),
        );
        let mut y = MapUnionWithTombstonesFstString::new_from(
            HashMap::from([
                ("banana".to_owned(), SetUnionHashSet::new_from([3])),
                ("cherry".to_owned(), SetUnionHashSet::new_from([4])),
            ]),
            FstTombstoneSet::new(),
        );

        // Add tombstone for "banana"
        y.as_reveal_mut().1.extend(vec!["banana".to_owned()]);

        x.merge(y);

        // Should have "apple" and "cherry", but not "banana" (tombstoned)
        assert!(!x.as_reveal_ref().0.contains_key("banana"));
        assert!(x.as_reveal_ref().0.contains_key("apple"));
        assert!(x.as_reveal_ref().0.contains_key("cherry"));
        assert!(x.as_reveal_ref().1.contains(b"banana"));
    }

    #[test]
    fn roaring_merge_efficiency() {
        // Test that merging roaring bitmaps works correctly
        let mut x = MapUnionWithTombstonesRoaring::new_from(
            HashMap::from([
                (1u64, SetUnionHashSet::new_from([1])),
                (2, SetUnionHashSet::new_from([2])),
            ]),
            RoaringTombstoneSet::from_iter(vec![10u64, 20]),
        );

        let y = MapUnionWithTombstonesRoaring::new_from(
            HashMap::from([(3u64, SetUnionHashSet::new_from([3]))]),
            RoaringTombstoneSet::from_iter(vec![30u64, 2]),
        );

        x.merge(y);

        // Should have all tombstones
        assert!(x.as_reveal_ref().1.contains(&10));
        assert!(x.as_reveal_ref().1.contains(&20));
        assert!(x.as_reveal_ref().1.contains(&30));
        assert!(x.as_reveal_ref().1.contains(&2));

        // Should not have key 2 in the map
        assert!(!x.as_reveal_ref().0.contains_key(&2));

        // Should have keys 1 and 3
        assert!(x.as_reveal_ref().0.contains_key(&1));
        assert!(x.as_reveal_ref().0.contains_key(&3));
    }
}
