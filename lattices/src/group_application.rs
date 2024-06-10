//! Module containing the [`Zset`] applications of groups.
use std::collections::HashMap;

use crate::{Addition, Inverse, Zero};

#[derive(PartialEq, Debug)]
/// Implementation of the Zset group (Z_A, +, 0, -Z_A) where A is the set of integers.

pub struct Zset(i32);

impl Zset {
    /// Create a new instance of Zset with the given value.
    pub fn new(value: i32) -> Self {
        Zset(value)
    }
}

/// Implementation of the addition trait for Zset group.
impl Addition<i32> for Zset {
    /// Integer addition is add function of Zset.
    fn add(&mut self, other: Zset) {
        self.0 += other.0
    }
}

/// Implementation of Identity for addition.
impl Zero<i32> for Zset {
    /// 0 is the zero element of the semiring.
    fn zero(&self) -> Zset {
        Zset(0)
    }
}

/// Implementation of Inverse for Zset group.
impl Inverse<i32> for Zset {
    /// Returns the inverse of the group element.
    fn inverse(&self) -> Zset {
        Zset(-self.0)
    }
}

#[allow(dead_code)]
/// Type alias for Zset map where keys are of type K and values are Zsets.
type ZsetMap<K> = HashMap<K, Zset>;

#[allow(dead_code)]
/// Type alias for indexed Zsets.
type IndexedZset<K, V> = HashMap<K, ZsetMap<V>>;

#[allow(dead_code)]
/// Flatmap function that converts an indexed Z-set into a Z-set of key-value pairs.
fn flatmap<K, V>(indexed_zset: IndexedZset<K, V>) -> ZsetMap<(K, V)>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone + Eq + std::hash::Hash,
{
    let mut result: ZsetMap<(K, V)> = HashMap::new();

    for (key, zset_map) in indexed_zset.into_iter() {
        for (sub_key, zset) in zset_map.into_iter() {
            let combined_key = (key.clone(), sub_key);
            result
                .entry(combined_key)
                .or_insert_with(|| Zset(0))
                .add(zset);
        }
    }

    result
}

#[allow(dead_code)]
/// Group sum function that combines grouping and summation using flatmap.
fn group_sum<K, V, F>(input: ZsetMap<V>, f: F) -> ZsetMap<K>
where
    K: Clone + Eq + std::hash::Hash,
    V: Clone + Eq + std::hash::Hash,
    F: Fn(&V) -> K,
{
    let mut indexed_zset: IndexedZset<K, V> = HashMap::new();

    // Grouping
    for (value, zset) in input.into_iter() {
        let key = f(&value);
        indexed_zset.entry(key).or_default().insert(value, zset);
    }

    // Flattening and summing
    let flat_zset = flatmap(indexed_zset);

    // Aggregating the flattened Z-set by summing values for each group
    let mut result: ZsetMap<K> = HashMap::new();
    for ((key, _), zset) in flat_zset.into_iter() {
        result.entry(key).or_insert_with(|| Zset(0)).add(zset);
    }

    result
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_sum() {
        // Create a ZsetMap with some initial values
        let mut zset_map: ZsetMap<String> = HashMap::new();
        zset_map.insert("joe".to_string(), Zset::new(1));
        zset_map.insert("anne".to_string(), Zset::new(-1));
        zset_map.insert("alice".to_string(), Zset::new(2));
        zset_map.insert("john".to_string(), Zset::new(1));
        zset_map.insert("jack".to_string(), Zset::new(3));

        // Define a key function: extracts first character in the name variable.
        let key_fn = |_name: &String| _name.chars().next().unwrap();

        // Perform group sum operation
        let grouped_sums = group_sum(zset_map, key_fn);

        let mut expected: ZsetMap<char> = HashMap::new();
        expected.insert('j', Zset::new(5)); // joe (1) + john (1) + jack (3)
        expected.insert('a', Zset::new(1)); // anne (-1) + alice (2)

        assert_eq!(grouped_sums, expected);
    }

    #[test]
    fn test_group_sum_empty() {
        // Create an empty ZsetMap
        let zset_map: ZsetMap<String> = HashMap::new();

        // Define a key function: extracts first character in the name variable.
        let key_fn = |name: &String| name.chars().next().unwrap();

        let grouped_sums = group_sum(zset_map, key_fn);

        let expected: ZsetMap<char> = HashMap::new();

        assert_eq!(grouped_sums, expected);
    }

    #[test]
    fn test_group_sum_single_group() {
        // Create a ZsetMap with values that all map to the same group
        let mut zset_map: ZsetMap<String> = HashMap::new();
        zset_map.insert("joe".to_string(), Zset::new(1));
        zset_map.insert("john".to_string(), Zset::new(2));
        zset_map.insert("jack".to_string(), Zset::new(3));

        // Define a key function
        let key_fn = |_name: &String| 'j'; // All keys will map to 'j'

        let grouped_sums = group_sum(zset_map, key_fn);

        let mut expected: ZsetMap<char> = HashMap::new();
        expected.insert('j', Zset::new(6)); // joe (1) + john (2) + jack (3)

        assert_eq!(grouped_sums, expected);
    }

    #[test]
    fn test_group_sum_negative_values() {
        // Create a ZsetMap with some negative values
        let mut zset_map: ZsetMap<String> = HashMap::new();
        zset_map.insert("joe".to_string(), Zset::new(-1));
        zset_map.insert("anne".to_string(), Zset::new(-1));
        zset_map.insert("alice".to_string(), Zset::new(1));
        zset_map.insert("john".to_string(), Zset::new(-1));

        // Define a key function: extracts first character in the name variable.
        let key_fn = |name: &String| name.chars().next().unwrap();

        let grouped_sums = group_sum(zset_map, key_fn);

        let mut expected: ZsetMap<char> = HashMap::new();
        expected.insert('j', Zset::new(-2)); // joe (-1) + john (-1)
        expected.insert('a', Zset::new(0)); // anne (-1) + alice (1)

        assert_eq!(grouped_sums, expected);
    }
}
