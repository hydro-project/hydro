//! A vector that supports efficient deletion without reordering all subsequent items.
use std::collections::HashMap;
use std::hash::Hash;
use std::iter::FusedIterator;

/// A vector that supports efficient deletion without reordering all subsequent items.
pub struct SparseVec<T> {
    items: Vec<Option<T>>,
    item_locs: HashMap<T, Vec<usize>>,
}

impl<T> Default for SparseVec<T> {
    fn default() -> Self {
        SparseVec {
            items: Vec::default(),
            item_locs: HashMap::default(),
        }
    }
}

impl<T: Clone + Eq + Hash> SparseVec<T> {
    /// Insert item into the vector, see `https://doc.rust-lang.org/std/vec/struct.Vec.html#method.push`
    pub fn push(&mut self, item: T) {
        self.items.push(Some(item.clone()));
        self.item_locs
            .entry(item)
            .or_insert(Vec::with_capacity(1))
            .push(self.items.len() - 1);
    }

    /// Delete all items of a specific value from this vector. This takes time proportional to the amount of items with that value in the vector, not the total size of the vector.
    pub fn delete(&mut self, item: &T) {
        if let Some(indices) = self.item_locs.remove(item) {
            for index in indices {
                self.items[index] = None;
            }
        }
    }

    /// Iterate through all items `&T` in the vector in order. Deleted items will not appear in the iteration.
    pub fn iter(&self) -> SparseVecIter<'_, T> {
        SparseVecIter { vec: self, idx: 0 }
    }
}

#[derive(Clone)]
pub struct SparseVecIter<'a, T> {
    vec: &'a SparseVec<T>,
    idx: usize,
}

impl<'a, T> Iterator for SparseVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < self.vec.items.len() {
            let item = &self.vec.items[self.idx];
            self.idx += 1;
            if let Some(item) = item {
                return Some(item);
            }
        }
        None
    }
}

impl<'a, T> FusedIterator for SparseVecIter<'a, T> {}

#[cfg(test)]
mod test {
    use super::*;

    fn collect<T: Eq + Hash + Clone>(sv: &SparseVec<T>) -> Vec<T> {
        sv.iter().cloned().collect()
    }

    #[test]
    fn basic() {
        let mut x = SparseVec::default();

        x.push(0);
        x.push(1);
        x.push(2);

        x.delete(&1);

        assert_eq!(collect(&x), vec![0, 2]);
    }

    #[test]
    fn can_implement_default_on_types_that_dont_implement_default() {
        struct NoDefault;

        let x = SparseVec::<NoDefault>::default();

        assert_eq!(x.items.len(), 0);
        assert_eq!(x.item_locs.len(), 0);
    }
}
