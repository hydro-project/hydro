use std::fmt;
use std::hash::{BuildHasher, Hash, RandomState};

use hashbrown::hash_table::{Entry, HashTable};

use crate::{PartialEqVariadic, VariadicExt, VecVariadic};

/// HashSet that stores Variadics of owned values but allows
/// for lookups with RefVariadics as well
#[derive(Clone)]
pub struct VariadicHashSet<T, S = RandomState> {
    table: HashTable<T>,
    hasher: S,
}

impl<T> VariadicHashSet<T> {
    /// Creates a new `VariadicHashSet` with a default hasher.
    pub fn new() -> Self {
        Self {
            table: HashTable::new(),
            hasher: RandomState::default(),
        }
    }
}

impl<T> Default for VariadicHashSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for VariadicHashSet<T>
where
    T: fmt::Debug + VariadicExt + PartialEqVariadic,
    for<'a> T::AsRefVar<'a>: Hash + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<T, S> VariadicHashSet<T, S>
where
    T: VariadicExt + PartialEqVariadic,
    for<'a> T::AsRefVar<'a>: Hash,
    S: BuildHasher,
{
    fn get_hash(hasher: &S, ref_var: T::AsRefVar<'_>) -> u64 {
        hasher.hash_one(ref_var)
        // let mut hasher = hasher.build_hasher();
        // ref_var.hash(&mut hasher);
        // hasher.finish()
    }

    /// given a RefVariadic lookup key, get a RefVariadic version of a tuple in the set
    pub fn get<'a>(&'a self, ref_var: T::AsRefVar<'_>) -> Option<&'a T> {
        let hash = Self::get_hash(&self.hasher, ref_var);
        self.table.find(hash, |item| {
            <T as PartialEqVariadic>::eq_ref(ref_var, item.as_ref_var())
        })
    }

    /// insert a tuple
    pub fn insert(&mut self, element: T) -> bool {
        let hash = Self::get_hash(&self.hasher, element.as_ref_var());
        let entry = self.table.entry(
            hash,
            |item| <T as PartialEqVariadic>::eq(&element, item),
            |item| Self::get_hash(&self.hasher, item.as_ref_var()),
        );
        match entry {
            Entry::Occupied(_occupied_entry) => false,
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(element);
                true
            }
        }
    }

    /// return the number of tuples in the set
    pub fn len(&self) -> usize {
        self.table.len()
    }

    /// return the number of tuples in the set
    pub fn is_empty(&self) -> bool {
        self.table.len() == 0
    }

    /// drain the set: iterate and remove the tuples without deallocating
    pub fn drain(&mut self) -> hashbrown::hash_table::Drain<'_, T> {
        self.table.drain()
    }

    /// iterate through the set
    pub fn iter(&self) -> impl Iterator<Item = T::AsRefVar<'_>> {
        self.table.iter().map(|item| item.as_ref_var())
    }
}

impl<T, S> IntoIterator for VariadicHashSet<T, S>
where
    T: VariadicExt + PartialEqVariadic,
{
    type Item = T;
    type IntoIter = hashbrown::hash_table::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.table.into_iter()
    }
}

impl<T, S> VariadicHashSet<T, S> {
    /// allocate a new VariadicHashSet with a specific hasher
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            table: HashTable::new(),
            hasher,
        }
    }
    /// allocate a new VariadicHashSet with a specific hasher and capacity
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self {
            table: HashTable::with_capacity(capacity),
            hasher,
        }
    }
}

// THIS CODE ADAPTED FROM hashbrown::HashMap
impl<K, S> Extend<K> for VariadicHashSet<K, S>
where
    K: Eq + Hash + PartialEqVariadic,
    S: BuildHasher,
    for<'a> K::AsRefVar<'a>: Hash,
    // for<'a> S::Hasher: Fn(&'a K) -> u64,
    // A: Allocator,
{
    // #[cfg_attr(feature = "inline-more", inline)]
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        // Keys may be already present or show multiple times in the iterator.
        // Reserve the entire hint lower bound if the map is empty.
        // Otherwise reserve half the hint (rounded up), so the map
        // will only resize twice in the worst case.
        let iter = iter.into_iter();
        // let reserve =
        if self.is_empty() {
            iter.size_hint().0
        } else {
            (iter.size_hint().0 + 1) / 2
        };
        // let hasher = self.hasher.build_hasher();
        // self.table.reserve(reserve, hasher);
        iter.for_each(move |k| {
            self.insert(k);
        });
    }
}

impl<T, S> PartialEq for VariadicHashSet<T, S>
where
    T: Eq + Hash + PartialEqVariadic,
    S: BuildHasher,
    for<'a> T::AsRefVar<'a>: Hash,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter().all(|key| other.get(key).is_some())
    }
}

impl<T, S> FromIterator<T> for VariadicHashSet<T, S>
where
    T: Eq + Hash + PartialEqVariadic,
    S: BuildHasher + Default,
    for<'a> T::AsRefVar<'a>: Hash,
    // A: Default + Allocator,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::with_hasher(Default::default());
        set.extend(iter);
        set
    }
}

/// Column storage for Variadic tuples of type Schema
/// An alternative to VariadicHashSet
#[derive(Default)]
pub struct VariadicColumnarSet<Schema>
where
    Schema: VariadicExt,
{
    columns: Schema::AsVec,
    last_offset: usize,
}

impl<Schema> VariadicColumnarSet<Schema>
where
    Schema: VariadicExt,
{
    /// Insert an element into the set
    pub fn insert(&mut self, element: Schema) -> bool {
        if self.last_offset == 0 {
            self.columns = element.as_vec()
        } else {
            self.columns.push(element);
        }
        self.last_offset += 1;
        true
    }

    /// Iterate over the elements of the set
    pub fn iter(&self) -> impl Iterator<Item = <Schema as VariadicExt>::AsRefVar<'_>> {
        self.columns.zip_vecs()
    }

    /// Return number of elements in the set
    pub fn len(&self) -> usize {
        self.last_offset
    }

    /// Return true if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// iterate and drain items from the set
    pub fn drain(&mut self) -> impl Iterator<Item = Schema> + '_ {
        self.last_offset = 0;
        self.columns.drain(0..)
    }
}

impl<T> fmt::Debug for VariadicColumnarSet<T>
where
    T: fmt::Debug + VariadicExt + PartialEqVariadic,
    for<'a> T::AsRefVar<'a>: Hash + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<Schema> IntoIterator for VariadicColumnarSet<Schema>
where
    Schema: PartialEqVariadic,
{
    type Item = Schema;
    type IntoIter = <Schema::AsVec as VecVariadic>::IntoZip;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.columns.into_zip()
    }
}

// THIS CODE ADAPTED FROM hashbrown::HashMap
impl<K> Extend<K> for VariadicColumnarSet<K>
where
    K: Eq + Hash + PartialEqVariadic,
    for<'a> K::AsRefVar<'a>: Hash,
    // for<'a> S::Hasher: Fn(&'a K) -> u64,
    // A: Allocator,
{
    // #[cfg_attr(feature = "inline-more", inline)]
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        // self.table.reserve(reserve, hasher);
        iter.for_each(move |k| {
            self.insert(k);
        });
    }

    // #[inline]
    // #[cfg(feature = "nightly")]
    // fn extend_one(&mut self, (k, v): (K, V)) {
    //     self.insert(k, v);
    // }

    // #[inline]
    // #[cfg(feature = "nightly")]
    // fn extend_reserve(&mut self, additional: usize) {
    //     // Keys may be already present or show multiple times in the iterator.
    //     // Reserve the entire hint lower bound if the map is empty.
    //     // Otherwise reserve half the hint (rounded up), so the map
    //     // will only resize twice in the worst case.
    //     let reserve = if self.is_empty() {
    //         additional
    //     } else {
    //         (additional + 1) / 2
    //     };
    //     self.reserve(reserve);
    // }
}
