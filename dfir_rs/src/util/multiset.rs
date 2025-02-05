//! A multiset backed by a HashMap
use std::collections::HashMap;
use std::hash::Hash;

/// A multiset backed by a HashMap
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct HashMultiSet<T: Hash + Eq> {
    items: HashMap<T, usize>,
    len: usize,
}

impl<T: Hash + Eq> HashMultiSet<T> {
    /// Insert item into the multiset. see `https://doc.rust-lang.org/std/collections/struct.HashSet.html#method.insert`
    pub fn insert(&mut self, value: T) {
        *self.items.entry(value).or_default() += 1;
        self.len += 1;
    }
}

impl<T> Default for HashMultiSet<T>
where
    T: Hash + Eq,
{
    fn default() -> Self {
        Self {
            items: HashMap::default(),
            len: 0,
        }
    }
}

impl<T> FromIterator<T> for HashMultiSet<T>
where
    T: Hash + Eq,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut ret = HashMultiSet::default();

        for item in iter {
            ret.insert(item);
        }

        ret
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic() {
        let mut x = HashMultiSet::default();

        x.insert(1);
        x.insert(2);
        x.insert(2);

        assert_eq!(x, HashMultiSet::from_iter([2, 1, 2]));
    }
}
