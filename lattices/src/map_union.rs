//! Map-union compound lattice.
//!
//! Each key corresponds to a lattice value instance. Merging map-union lattices is done by
//! unioning the keys and merging the values of intersecting keys.

use std::cmp::Ordering::*;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

use crate::cc_traits::{GetMut, Keyed, Map, MapIter, SimpleKeyedRef};
use crate::collections::{ArrayMap, SingletonMap, VecMap};
use crate::{ConvertFrom, LatticeOrd, Merge};

/// A map-union lattice.
///
/// `Tag` specifies what datastructure to use, allowing us to deal with different datastructures
/// generically.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MapUnion<Map>(pub Map);
impl<Map> MapUnion<Map> {
    /// Create a new `MapUnion` from a `Map`.
    pub fn new(val: Map) -> Self {
        Self(val)
    }

    /// Create a new `MapUnion` from an `Into<Map>`.
    pub fn new_from(val: impl Into<Map>) -> Self {
        Self::new(val.into())
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther> Merge<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Keyed<Key = K, Item = ValSelf>
        + Extend<(K, ValSelf)>
        + for<'a> GetMut<&'a K, Item = ValSelf>,
    MapOther: IntoIterator<Item = (K, ValOther)>,
    ValSelf: Merge<ValOther> + ConvertFrom<ValOther>,
{
    fn merge(&mut self, other: MapUnion<MapOther>) -> bool {
        let mut changed = false;
        // This vec collect is needed to prevent simultaneous mut references `self.0.extend` and
        // `self.0.get_mut`.
        // TODO(mingwei): This could be fixed with a different structure, maybe some sort of
        // `Collection` entry API.
        let iter: Vec<_> = other
            .0
            .into_iter()
            .filter_map(|(k_other, val_other)| {
                match self.0.get_mut(&k_other) {
                    // Key collision, merge into `self`.
                    Some(mut val_self) => {
                        changed |= val_self.merge(val_other);
                        None
                    }
                    // New value, convert for extending.
                    None => {
                        changed = true;
                        Some((k_other, ValSelf::from(val_other)))
                    }
                }
            })
            .collect();
        self.0.extend(iter);
        changed
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther> ConvertFrom<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Keyed<Key = K, Item = ValSelf> + FromIterator<(K, ValSelf)>,
    MapOther: IntoIterator<Item = (K, ValOther)>,
    ValSelf: ConvertFrom<ValOther>,
{
    fn from(other: MapUnion<MapOther>) -> Self {
        Self(
            other
                .0
                .into_iter()
                .map(|(k_other, val_other)| (k_other, ConvertFrom::from(val_other)))
                .collect(),
        )
    }
}

impl<MapSelf, MapOther, K, ValSelf, ValOther> PartialOrd<MapUnion<MapOther>> for MapUnion<MapSelf>
where
    MapSelf: Map<K, ValSelf, Key = K, Item = ValSelf> + MapIter + SimpleKeyedRef,
    MapOther: Map<K, ValOther, Key = K, Item = ValOther> + MapIter + SimpleKeyedRef,
    ValSelf: PartialOrd<ValOther>,
{
    fn partial_cmp(&self, other: &MapUnion<MapOther>) -> Option<std::cmp::Ordering> {
        let mut self_any_greater = false;
        let mut other_any_greater = false;
        for k in self
            .0
            .iter()
            .map(|(k, _v)| <MapSelf as SimpleKeyedRef>::into_ref(k))
            .chain(
                other
                    .0
                    .iter()
                    .map(|(k, _v)| <MapOther as SimpleKeyedRef>::into_ref(k)),
            )
        {
            match (self.0.get(k), other.0.get(k)) {
                (Some(self_value), Some(other_value)) => {
                    match self_value.partial_cmp(&*other_value) {
                        None => {
                            return None;
                        }
                        Some(Less) => {
                            other_any_greater = true;
                        }
                        Some(Greater) => {
                            self_any_greater = true;
                        }
                        Some(Equal) => {}
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
            // We check this one after each loop iteration.
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
    ValSelf: PartialEq<ValOther>,
{
    fn eq(&self, other: &MapUnion<MapOther>) -> bool {
        if self.0.len() != other.0.len() {
            return false;
        }

        for k in self
            .0
            .iter()
            .map(|(k, _v)| <MapSelf as SimpleKeyedRef>::into_ref(k))
            .chain(
                other
                    .0
                    .iter()
                    .map(|(k, _v)| <MapOther as SimpleKeyedRef>::into_ref(k)),
            )
        {
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
impl<MapSelf> Eq for MapUnion<MapSelf>
where
    Self: PartialEq,
    MapSelf: Eq,
{
}

/// [`std::collections::HashMap`]-backed [`MapUnion`] lattice.
pub type MapUnionHashMap<K, Val> = MapUnion<HashMap<K, Val>>;

/// [`std::collections::BTreeMap`]-backed [`MapUnion`] lattice.
pub type MapUnionBTreeMap<K, Val> = MapUnion<BTreeMap<K, Val>>;

/// [`Vec`]-backed [`MapUnion`] lattice.
pub type MapUnionVec<K, Val> = MapUnion<VecMap<K, Val>>;

/// Array-backed [`MapUnion`] lattice.
pub type MapUnionArrayMap<K, Val, const N: usize> = MapUnion<ArrayMap<K, Val, N>>;

/// [`crate::collections::SingletonMap`]-backed [`MapUnion`] lattice.
pub type MapUnionSingletonMap<K, Val> = MapUnion<SingletonMap<K, Val>>;

/// [`Option`]-backed [`MapUnion`] lattice.
pub type MapUnionOption<K, Val> = MapUnion<Option<(K, Val)>>;

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::collections::{SingletonMap, SingletonSet};
    use crate::set_union::{SetUnionHashSet, SetUnionSingletonSet};
    use crate::test::check_all;

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

    #[test]
    fn consistency() {
        let mut test_vec = Vec::new();

        for key in [0, 1, 2] {
            for value in [vec![], vec![0], vec![1], vec![0, 1]] {
                test_vec.push(MapUnionHashMap::new_from(HashMap::from_iter([(
                    key,
                    SetUnionHashSet::new_from(HashSet::from_iter(value.clone())),
                )])));
            }
        }

        check_all(&test_vec);
    }
}
