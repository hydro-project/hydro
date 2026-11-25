//! Module containing the [`MapUnion`] lattice and aliases for different datastructures.

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
use core::cmp::Ordering::{self, *};
use core::fmt::Debug;
use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::collections::HashMap;

use cc_traits::{Collection, GetKeyValue, Iter, MapInsert, SimpleCollectionRef};

#[cfg(feature = "alloc")]
use crate::cc_traits::GetMut;
use crate::cc_traits::{Keyed, Map, MapIter, SimpleKeyedRef};
#[cfg(feature = "alloc")]
use crate::collections::VecMap;
use crate::collections::{ArrayMap, MapMapValues, OptionMap, SingletonMap};
#[cfg(feature = "alloc")]
use crate::{Atomize, Merge};
use crate::{DeepReveal, IsBot, IsTop, LatticeBimorphism, LatticeFrom, LatticeOrd};

/// Map-union compound lattice.
///
/// Each key corresponds to a lattice value instance. Merging map-union lattices is done by
/// unioning the keys and merging the values of intersecting keys.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MapUnion<Map>(Map);
impl<Map> MapUnion<Map> {
    /// Create a new `MapUnion` from a `Map`.
    pub fn new(val: Map) -> Self {
        Self(val)
    }

    /// Create a new `MapUnion` from an `Into<Map>`.
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

impl<Map, Val> DeepReveal for MapUnion<Map>
where
    Map: Keyed<Item = Val> + MapMapValues<Val>,
    Val: DeepReveal,
{
    type Revealed = Map::MapValue<Val::Revealed>;

    fn deep_reveal(self) -> Self::Revealed {
        self.0.map_values(DeepReveal::deep_reveal)
    }
}

#[cfg(feature = "alloc")]
impl<MapSelf, MapOther, K, ValSelf, ValOther> Merge<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Keyed<Key = K, Item = ValSelf>
        + Extend<(K, ValSelf)>
        + for<'a> GetMut<&'a K, Item = ValSelf>,
    MapOther: IntoIterator<Item = (K, ValOther)>,
    ValSelf: Merge<ValOther> + LatticeFrom<ValOther>,
    ValOther: IsBot,
{
    fn merge(&mut self, other: MapUnion<MapOther>) -> bool {
        use alloc::vec::Vec;

        let mut changed = false;
        // This vec collect is needed to prevent simultaneous mut references `self.0.extend` and
        // `self.0.get_mut`.
        // TODO(mingwei): This could be fixed with a different structure, maybe some sort of
        // `Collection` entry API.
        let iter: Vec<_> = other
            .0
            .into_iter()
            .filter(|(_k_other, val_other)| !val_other.is_bot())
            .filter_map(|(k_other, val_other)| {
                if let Some(mut val_self) = self.0.get_mut(&k_other) {
                    // Key collision, merge into `self`.
                    changed |= val_self.merge(val_other);
                    None
                } else {
                    // New value, convert for extending.
                    changed = true;
                    Some((k_other, ValSelf::lattice_from(val_other)))
                }
            })
            .collect();
        self.0.extend(iter);
        changed
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther> LatticeFrom<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Keyed<Key = K, Item = ValSelf> + FromIterator<(K, ValSelf)>,
    MapOther: IntoIterator<Item = (K, ValOther)>,
    ValSelf: LatticeFrom<ValOther>,
{
    fn lattice_from(other: MapUnion<MapOther>) -> Self {
        Self(
            other
                .0
                .into_iter()
                .map(|(k_other, val_other)| (k_other, LatticeFrom::lattice_from(val_other)))
                .collect(),
        )
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther> PartialOrd<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Map<K, ValSelf, Key = K, Item = ValSelf> + MapIter + SimpleKeyedRef,
    MapOther: Map<K, ValOther, Key = K, Item = ValOther> + MapIter + SimpleKeyedRef,
    ValSelf: PartialOrd<ValOther> + IsBot,
    ValOther: IsBot,
{
    fn partial_cmp(&self, other: &MapUnion<MapOther>) -> Option<Ordering> {
        let mut self_any_greater = false;
        let mut other_any_greater = false;
        let self_keys = self
            .0
            .iter()
            .filter(|(_k, v)| !v.is_bot())
            .map(|(k, _v)| <MapSelf as SimpleKeyedRef>::into_ref(k));
        let other_keys = other
            .0
            .iter()
            .filter(|(_k, v)| !v.is_bot())
            .map(|(k, _v)| <MapOther as SimpleKeyedRef>::into_ref(k));
        for k in self_keys.chain(other_keys) {
            match (self.0.get(k), other.0.get(k)) {
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
        match (self_any_greater, other_any_greater) {
            (true, false) => Some(Greater),
            (false, true) => Some(Less),
            (false, false) => Some(Equal),
            // We check this one after each loop iteration above.
            (true, true) => unreachable!(),
        }
    }
}
impl<MapSelf, MapOther> LatticeOrd<MapUnion<MapOther>> for MapUnion<MapSelf> where
    Self: PartialOrd<MapUnion<MapOther>>
{
}

impl<MapSelf, MapOther, K, ValSelf, ValOther> PartialEq<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Map<K, ValSelf, Key = K, Item = ValSelf> + MapIter + SimpleKeyedRef,
    MapOther: Map<K, ValOther, Key = K, Item = ValOther> + MapIter + SimpleKeyedRef,
    ValSelf: PartialEq<ValOther> + IsBot,
    ValOther: IsBot,
{
    fn eq(&self, other: &MapUnion<MapOther>) -> bool {
        let self_keys = self
            .0
            .iter()
            .filter(|(_k, v)| !v.is_bot())
            .map(|(k, _v)| <MapSelf as SimpleKeyedRef>::into_ref(k));
        let other_keys = other
            .0
            .iter()
            .filter(|(_k, v)| !v.is_bot())
            .map(|(k, _v)| <MapOther as SimpleKeyedRef>::into_ref(k));
        for k in self_keys.chain(other_keys) {
            match (self.0.get(k), other.0.get(k)) {
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
impl<MapSelf> Eq for MapUnion<MapSelf> where Self: PartialEq {}

impl<Map> IsBot for MapUnion<Map>
where
    Map: Iter,
    Map::Item: IsBot,
{
    fn is_bot(&self) -> bool {
        self.0.iter().all(|v| v.is_bot())
    }
}

impl<Map> IsTop for MapUnion<Map> {
    fn is_top(&self) -> bool {
        false
    }
}

#[cfg(feature = "alloc")]
impl<Map, K, Val> Atomize for MapUnion<Map>
where
    Map: 'static
        + IntoIterator<Item = (K, Val)>
        + Keyed<Key = K, Item = Val>
        + Extend<(K, Val)>
        + for<'a> GetMut<&'a K, Item = Val>,
    K: 'static + Clone,
    Val: 'static + Atomize + LatticeFrom<<Val as Atomize>::Atom>,
{
    type Atom = MapUnionSingletonMap<K, Val::Atom>;

    // TODO: use impl trait, then remove 'static.
    type AtomIter = alloc::boxed::Box<dyn Iterator<Item = Self::Atom>>;

    fn atomize(self) -> Self::AtomIter {
        alloc::boxed::Box::new(self.0.into_iter().flat_map(|(k, val)| {
            val.atomize()
                .map(move |v| MapUnionSingletonMap::new_from((k.clone(), v)))
        }))
    }
}

/// [`std::collections::HashMap`]-backed [`MapUnion`] lattice.
#[cfg(feature = "std")]
pub type MapUnionHashMap<K, Val> = MapUnion<HashMap<K, Val>>;

/// [`std::collections::BTreeMap`]-backed [`MapUnion`] lattice.
#[cfg(feature = "alloc")]
pub type MapUnionBTreeMap<K, Val> = MapUnion<BTreeMap<K, Val>>;

/// [`Vec`](alloc::vec::Vec)-backed [`MapUnion`] lattice.
#[cfg(feature = "alloc")]
pub type MapUnionVec<K, Val> = MapUnion<VecMap<K, Val>>;

/// Array-backed [`MapUnion`] lattice.
pub type MapUnionArrayMap<K, Val, const N: usize> = MapUnion<ArrayMap<K, Val, N>>;

/// [`crate::collections::SingletonMap`]-backed [`MapUnion`] lattice.
pub type MapUnionSingletonMap<K, Val> = MapUnion<SingletonMap<K, Val>>;

/// [`Option`]-backed [`MapUnion`] lattice.
pub type MapUnionOptionMap<K, Val> = MapUnion<OptionMap<K, Val>>;

/// Composable bimorphism, wraps an existing morphism by partitioning it per key.
///
/// For example, `KeyedBimorphism<..., CartesianProduct<...>>` is a join.
///
/// A missing key is lattice bottom (`Default::default`). Keys that exist on only one
/// side are still paired when that side is not itself bottom, so a nested bottom kept
/// in one representation and collapsed by `Merge` in another cannot change the result.
pub struct KeyedBimorphism<MapOut, Bimorphism> {
    bimorphism: Bimorphism,
    _phantom: PhantomData<fn() -> MapOut>,
}
impl<MapOut, Bimorphism> KeyedBimorphism<MapOut, Bimorphism> {
    /// Create a `KeyedBimorphism` using `bimorphism` for handling values.
    pub fn new(bimorphism: Bimorphism) -> Self {
        Self {
            bimorphism,
            _phantom: PhantomData,
        }
    }
}
impl<MapA, MapB, MapOut, ValFunc, K> LatticeBimorphism<MapUnion<MapA>, MapUnion<MapB>>
    for KeyedBimorphism<MapOut, ValFunc>
where
    ValFunc: LatticeBimorphism<MapA::Item, MapB::Item>,
    // `K` avoids `MapA: GetKeyValue<&'a MapA::Key>`, which is a bounds cycle.
    K: Clone + Eq,
    MapA: MapIter
        + SimpleKeyedRef
        + SimpleCollectionRef
        + Keyed<Key = K>
        + for<'a> GetKeyValue<&'a K>,
    MapB: MapIter
        + SimpleKeyedRef
        + SimpleCollectionRef
        + Keyed<Key = K>
        + for<'a> GetKeyValue<&'a K>,
    MapA::Item: Clone + Default + IsBot,
    MapB::Item: Clone + Default + IsBot,
    MapOut: Default + MapInsert<K> + Collection<Item = ValFunc::Output>,
    ValFunc::Output: IsBot,
{
    type Output = MapUnion<MapOut>;

    fn call(&mut self, lat_a: MapUnion<MapA>, lat_b: MapUnion<MapB>) -> Self::Output {
        let mut output = MapUnion::<MapOut>::default();
        let map_a = lat_a.as_reveal_ref();
        let map_b = lat_b.as_reveal_ref();

        for (key, val_a) in map_a.iter() {
            let key = <MapA as SimpleKeyedRef>::into_ref(key);
            let val_a = <MapA as SimpleCollectionRef>::into_ref(val_a);
            let val_b = if let Some((_key, val_b)) = map_b.get_key_value(key) {
                <MapB as SimpleCollectionRef>::into_ref(val_b).clone()
            } else if !val_a.is_bot() {
                <MapB::Item as Default>::default()
            } else {
                continue;
            };
            let val_out = LatticeBimorphism::call(&mut self.bimorphism, val_a.clone(), val_b);
            if !val_out.is_bot() {
                <MapOut as MapInsert<_>>::insert(output.as_reveal_mut(), key.clone(), val_out);
            }
        }

        for (key, val_b) in map_b.iter() {
            let key = <MapB as SimpleKeyedRef>::into_ref(key);
            if map_a.get_key_value(key).is_some() {
                continue;
            }
            let val_b = <MapB as SimpleCollectionRef>::into_ref(val_b);
            if val_b.is_bot() {
                continue;
            }
            let val_out = LatticeBimorphism::call(
                &mut self.bimorphism,
                <MapA::Item as Default>::default(),
                val_b.clone(),
            );
            if !val_out.is_bot() {
                <MapOut as MapInsert<_>>::insert(output.as_reveal_mut(), key.clone(), val_out);
            }
        }
        output
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::collections::SingletonSet;
    use crate::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
    use crate::test::{cartesian_power, check_all, check_atomize_each, check_lattice_bimorphism};

    #[test]
    fn test_map_union() {
        let mut my_map_a = <MapUnionHashMap<&str, SetUnionHashSet<u64>>>::default();
        let my_map_b = <MapUnionSingletonMap<&str, SetUnionSingletonSet<u64>>>::new(SingletonMap(
            "hello",
            SetUnionSingletonSet::new(SingletonSet(100)),
        ));
        let my_map_c =
            MapUnionSingletonMap::new_from(("hello", SetUnionHashSet::new_from([100, 200])));
        my_map_a.merge(my_map_b);
        my_map_a.merge(my_map_c);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn consistency_atomize() {
        use alloc::vec;
        use alloc::vec::Vec;

        let mut test_vec = Vec::new();

        // Size 0.
        test_vec.push(MapUnionHashMap::default());
        // Size 1.
        for key in [0, 1] {
            for value in [vec![], vec![0], vec![1], vec![0, 1]] {
                test_vec.push(MapUnionHashMap::new(HashMap::from_iter([(
                    key,
                    SetUnionHashSet::new(HashSet::from_iter(value)),
                )])));
            }
        }
        // Size 2.
        for [val_a, val_b] in cartesian_power(&[vec![], vec![0], vec![1], vec![0, 1]]) {
            test_vec.push(MapUnionHashMap::new(HashMap::from_iter([
                (0, SetUnionHashSet::new(HashSet::from_iter(val_a.clone()))),
                (1, SetUnionHashSet::new(HashSet::from_iter(val_b.clone()))),
            ])));
        }

        check_all(&test_vec);
        check_atomize_each(&test_vec);
    }

    /// Check that a key with a value of bottom is the same as an empty map, etc.
    #[test]
    fn test_collapes_bot() {
        let map_empty = <MapUnionHashMap<&str, SetUnionHashSet<u64>>>::default();
        let map_a_bot = <MapUnionSingletonMap<&str, SetUnionHashSet<u64>>>::new(SingletonMap(
            "a",
            Default::default(),
        ));
        let map_b_bot = <MapUnionSingletonMap<&str, SetUnionHashSet<u64>>>::new(SingletonMap(
            "b",
            Default::default(),
        ));

        assert_eq!(map_empty, map_a_bot);
        assert_eq!(map_empty, map_b_bot);
        assert_eq!(map_a_bot, map_b_bot);
    }

    /// A nested `()` kept under a key equals that key being absent after `Merge`
    /// drops bottoms. Both argument orders must agree, including when the only
    /// non-bottom value is on the other map.
    #[test]
    fn test_double_keyed() {
        use std::collections::HashMap;

        use crate::{IsBot, Pair, PairBimorphism};

        let system_table_item = MapUnionSingletonMap::new_from((
            "members",
            MapUnionHashMap::new_from([("key", SetUnionHashSet::new_from(["value"]))]),
        ));
        let mut system_table_state: MapUnionHashMap<
            &str,
            MapUnionHashMap<&str, SetUnionHashSet<&str>>,
        > = Default::default();
        system_table_state.merge(system_table_item.clone());

        // Explicit nested bottom. Not inserted via `merge`, which would delete it.
        let mut gets_inner: MapUnionHashMap<&str, ()> = Default::default();
        gets_inner.as_reveal_mut().insert("key", ());
        let gets_item = MapUnionSingletonMap::new_from(("members", gets_inner));
        let mut gets_state: MapUnionHashMap<&str, MapUnionHashMap<&str, ()>> = Default::default();
        gets_state.merge(gets_item.clone());

        let mut lattice_bimorphism = KeyedBimorphism::<
            HashMap<&str, MapUnionHashMap<&str, Pair<SetUnionHashSet<&str>, ()>>>,
            _,
        >::new(KeyedBimorphism::<
            HashMap<&str, Pair<SetUnionHashSet<&str>, ()>>,
            _,
        >::new(PairBimorphism));
        let from_state_and_item =
            lattice_bimorphism.call(system_table_state.clone(), gets_item.clone());
        let from_item_and_state =
            lattice_bimorphism.call(system_table_item.clone(), gets_state.clone());

        let expected = MapUnionHashMap::new_from([(
            "members",
            MapUnionHashMap::new_from([(
                "key",
                Pair::new(SetUnionHashSet::new_from(["value"]), ()),
            )]),
        )]);
        assert_eq!(
            from_state_and_item, from_item_and_state,
            "deep-bottom orientations diverged"
        );
        assert_eq!(from_state_and_item, expected);
        assert!(!from_state_and_item.is_bot());

        // The non-bottom payload sits only on `lat_b` when the collapsed bottom is `lat_a`.
        let mut swapped = KeyedBimorphism::<
            HashMap<&str, MapUnionHashMap<&str, Pair<(), SetUnionHashSet<&str>>>>,
            _,
        >::new(KeyedBimorphism::<
            HashMap<&str, Pair<(), SetUnionHashSet<&str>>>,
            _,
        >::new(PairBimorphism));
        let swapped_explicit = swapped.call(gets_item, system_table_item);
        let swapped_collapsed = swapped.call(gets_state, system_table_state);
        let expected_swapped = MapUnionHashMap::new_from([(
            "members",
            MapUnionHashMap::new_from([(
                "key",
                Pair::new((), SetUnionHashSet::new_from(["value"])),
            )]),
        )]);
        assert_eq!(
            swapped_explicit, swapped_collapsed,
            "key present only on the other side was dropped"
        );
        assert_eq!(swapped_explicit, expected_swapped);
        assert!(!swapped_explicit.is_bot());
    }

    #[test]
    fn test_join_aka_keyed_cartesian_product() {
        let items_a = &[
            MapUnionHashMap::new_from([("foo", SetUnionHashSet::new_from(["bar"]))]),
            MapUnionHashMap::new_from([("foo", SetUnionHashSet::new_from(["baz"]))]),
            MapUnionHashMap::new_from([("hello", SetUnionHashSet::new_from(["world"]))]),
        ];
        let items_b = &[
            MapUnionHashMap::new_from([("foo", SetUnionHashSet::new_from(["bang"]))]),
            MapUnionHashMap::new_from([(
                "hello",
                SetUnionHashSet::new_from(["goodbye", "farewell"]),
            )]),
        ];

        check_lattice_bimorphism(
            KeyedBimorphism::<HashMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_a,
            items_a,
        );
        check_lattice_bimorphism(
            KeyedBimorphism::<HashMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_a,
            items_b,
        );
        check_lattice_bimorphism(
            KeyedBimorphism::<HashMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_b,
            items_a,
        );
        check_lattice_bimorphism(
            KeyedBimorphism::<HashMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_b,
            items_b,
        );

        check_lattice_bimorphism(
            KeyedBimorphism::<BTreeMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_a,
            items_a,
        );
        check_lattice_bimorphism(
            KeyedBimorphism::<BTreeMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_a,
            items_b,
        );
        check_lattice_bimorphism(
            KeyedBimorphism::<BTreeMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_b,
            items_a,
        );
        check_lattice_bimorphism(
            KeyedBimorphism::<BTreeMap<_, _>, _>::new(
                CartesianProductBimorphism::<HashSet<_>>::default(),
            ),
            items_b,
            items_b,
        );
    }
}
