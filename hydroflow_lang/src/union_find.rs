//! Union-find data structure, see [`UnionFind`].

use slotmap::{Key, SecondaryMap};

/// Union-find data structure.
///
/// Used to efficiently track sets of equivalent items.
///
/// <https://en.wikipedia.org/wiki/Disjoint-set_data_structure>
#[derive(Default, Clone)]
pub struct UnionFind<K>
where
    K: Key,
{
    links: SecondaryMap<K, K>,
}
impl<K> UnionFind<K>
where
    K: Key,
{
    /// Creates a new `UnionFind`, same as [`Default::default()`].
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates a new `UnionFind` with the given key capacity pre-allocated.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            links: SecondaryMap::with_capacity(capacity),
        }
    }

    /// Combines two items `a` and `b` as equivalent, in the same set.
    pub fn union(&mut self, a: K, b: K) {
        let i = self.find(a);
        let j = self.find(b);
        if i == j {
            return;
        }
        self.links[i] = j;
    }

    /// Finds the "representative" item for `k`. Each set of equivalent items is represented by one
    /// of its member items.
    pub fn find(&mut self, k: K) -> K {
        if let Some(next) = self.links.insert(k, k) {
            if k == next {
                return k;
            }
            self.links[k] = self.find(next);
        }
        self.links[k]
    }

    /// Returns if `a` and `b` are equivalent, i.e. in the same set.
    pub fn same_set(&mut self, a: K, b: K) -> bool {
        self.find(a) == self.find(b)
    }
}

#[cfg(test)]
mod test {
    use slotmap::SlotMap;

    use super::*;

    #[test]
    fn test_basic() {
        let mut sm = SlotMap::new();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let d = sm.insert(());

        let mut uf = UnionFind::new();
        assert!(!uf.same_set(a, b));
        uf.union(a, b);
        assert!(uf.same_set(a, b));
        uf.union(c, a);
        assert!(uf.same_set(b, c));

        assert!(!uf.same_set(a, d));
        assert!(!uf.same_set(b, d));
        assert!(!uf.same_set(d, c));
    }
}
