use std::fmt;
use std::hash::{BuildHasher, Hash, RandomState};

use hashbrown::hash_table::{Entry, HashTable};
use sealed::sealed;

use crate::{PartialEqVariadic, VariadicExt, VecVariadic};

/// Trait for a set of Tuples
pub trait VariadicSet {
    /// The Schema (aka Variadic type) associated with tuples in this set
    type Schema: PartialEqVariadic;

    /// Insert an element into the set
    fn insert(&mut self, element: Self::Schema) -> bool;

    /// Iterate over the elements of the set
    fn iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    /// Return number of elements in the set
    fn len(&self) -> usize;

    /// Return true if empty
    fn is_empty(&self) -> bool;

    /// iterate and drain items from the set
    fn drain(&mut self) -> impl Iterator<Item = Self::Schema>;

    /// Check for containment
    fn contains(&self, value: <Self::Schema as VariadicExt>::AsRefVar<'_>) -> bool;
}

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
    T: PartialEqVariadic,
    for<'a> T::AsRefVar<'a>: Hash,
    S: BuildHasher,
{
    /// given a RefVariadic lookup key, get a RefVariadic version of a tuple in the set
    pub fn get<'a>(&'a self, ref_var: T::AsRefVar<'_>) -> Option<&'a T> {
        let hash = self.hasher.hash_one(ref_var);
        self.table.find(hash, |item| {
            <T as PartialEqVariadic>::eq_ref(ref_var, item.as_ref_var())
        })
    }
}
impl<T, S> VariadicSet for VariadicHashSet<T, S>
where
    T: VariadicExt + PartialEqVariadic,
    for<'a> T::AsRefVar<'a>: Hash,
    S: BuildHasher,
{
    type Schema = T;

    /// insert a tuple
    fn insert(&mut self, element: T) -> bool {
        // let hash = Self::get_hash(&self.hasher, element.as_ref_var());
        let hash = self.hasher.hash_one(element.as_ref_var());
        let entry = self.table.entry(
            hash,
            |item| <T as PartialEqVariadic>::eq(&element, item),
            |item| self.hasher.hash_one(item.as_ref_var()),
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
    fn len(&self) -> usize {
        self.table.len()
    }

    /// return the number of tuples in the set
    fn is_empty(&self) -> bool {
        self.table.len() == 0
    }

    /// drain the set: iterate and remove the tuples without deallocating
    // fn drain(&mut self) -> hashbrown::hash_table::Drain<'_, T> {
    fn drain(&mut self) -> impl Iterator<Item = Self::Schema> {
        self.table.drain()
    }

    fn contains(&self, value: <Self::Schema as VariadicExt>::AsRefVar<'_>) -> bool {
        self.get(value).is_some()
    }

    /// iterate through the set
    fn iter(&self) -> impl Iterator<Item = T::AsRefVar<'_>> {
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

/// Trait for a multiset of Tuples
#[sealed]
pub trait VariadicMultiset {
    /// The Schema (aka Variadic type) associated with tuples in this set
    type Schema: PartialEqVariadic;

    /// Insert an element into the set
    fn insert(&mut self, element: Self::Schema);

    /// Iterate over the elements of the set
    fn iter(&self) -> impl Iterator<Item = <Self::Schema as VariadicExt>::AsRefVar<'_>>;

    /// Return number of elements in the set
    fn len(&self) -> usize;

    /// Return true if empty
    fn is_empty(&self) -> bool;

    /// iterate and drain items from the set
    fn drain(&mut self) -> impl Iterator<Item = Self::Schema>;

    /// Check for containment
    fn contains(&self, value: <Self::Schema as VariadicExt>::AsRefVar<'_>) -> bool;
}

/// HashMap keyed on Variadics of owned values but allows
/// for lookups with RefVariadics as well
#[derive(Clone)]
pub struct VariadicCountedHashSet<K, S = RandomState>
where
    K: VariadicExt,
{
    table: HashTable<(K, usize)>,
    hasher: S,
    len: usize,
}

impl<K> VariadicCountedHashSet<K>
where
    K: VariadicExt,
{
    /// Creates a new `VariadicCountedHashSet` with a default hasher.
    pub fn new() -> Self {
        Self {
            table: HashTable::new(),
            hasher: RandomState::default(),
            len: 0,
        }
    }
}

impl<K> Default for VariadicCountedHashSet<K>
where
    K: VariadicExt,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> fmt::Debug for VariadicCountedHashSet<K>
where
    K: fmt::Debug + VariadicExt + PartialEqVariadic,
    for<'a> K::AsRefVar<'a>: Hash + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.table.iter()).finish()
    }
}

impl<K, S> VariadicCountedHashSet<K, S>
where
    K: PartialEqVariadic,
    for<'a> K::AsRefVar<'a>: Hash,
    S: BuildHasher,
{
    /// given a RefVariadic lookup key, get a RefVariadic version of an entry in the map
    pub fn get<'a>(&'a self, ref_var: K::AsRefVar<'_>) -> Option<&'a (K, usize)> {
        let hash = self.hasher.hash_one(ref_var);
        self.table.find(hash, |(key, _val)| {
            <K as PartialEqVariadic>::eq_ref(ref_var, key.as_ref_var())
        })
    }
}

#[sealed]
impl<K, S> VariadicMultiset for VariadicCountedHashSet<K, S>
where
    K: VariadicExt + PartialEqVariadic + Hash + Clone,
    for<'a> K::AsRefVar<'a>: Hash,
    S: BuildHasher,
{
    type Schema = K;

    /// insert a tuple
    fn insert(&mut self, element: K) {
        let hash = self.hasher.hash_one(element.as_ref_var());
        self.table
            .entry(
                hash,
                |item| <K as PartialEqVariadic>::eq(&element, &item.0),
                |item| self.hasher.hash_one((&item.0, item.1 + 1)),
            )
            .and_modify(|(_, count)| *count += 1)
            .or_insert((element, 1));
        self.len += 1;
    }

    /// return the number of tuples in the multiset
    fn len(&self) -> usize {
        self.len
    }

    /// return the number of tuples in the multiset
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// drain the multiset: iterate and remove the tuples without deallocating
    fn drain(&mut self) -> impl Iterator<Item = Self::Schema> {
        // TODO: this shouldn't clone the last copy of each k!
        // particularly bad when there's typically only 1 copy per item
        self.table
            .drain()
            .flat_map(|(k, num)| (0..num).map(move |_i| k.clone()))
    }

    fn contains(&self, value: <Self::Schema as VariadicExt>::AsRefVar<'_>) -> bool {
        self.get(value).is_some()
    }

    /// iterate through the multiset
    fn iter(&self) -> impl Iterator<Item = K::AsRefVar<'_>> {
        self.table
            .iter()
            .flat_map(|(k, num)| (0..*num).map(move |_i| k.as_ref_var()))
    }
}

impl<T> IntoIterator for VariadicCountedHashSet<T>
where
    T: VariadicExt + PartialEqVariadic + Clone,
{
    type Item = T;
    type IntoIter = DuplicateCounted<hashbrown::hash_table::IntoIter<(T, usize)>, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        DuplicateCounted {
            iter: self.table.into_iter(),
            state: None,
        }
    }
}

/// Iterator helper for [`VariadicCountedHashSet::into_iter`].
pub struct DuplicateCounted<Iter, Item> {
    iter: Iter,
    state: Option<(Item, usize)>,
    // count: usize,
}
impl<Iter, Item> Iterator for DuplicateCounted<Iter, Item>
where
    Iter: Iterator<Item = (Item, usize)>,
    Item: Clone,
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state.take() {
                Some((item, 1)) => {
                    self.state = None;
                    return Some(item);
                }
                None | Some((_, 0)) => match self.iter.next() {
                    Some(state) => self.state = Some(state),
                    None => return None,
                },
                Some((item, many)) => {
                    let out = Some(item.clone());
                    self.state = Some((item, many - 1));
                    return out;
                }
            }
        }
    }
}

impl<K, S> VariadicCountedHashSet<K, S>
where
    K: VariadicExt,
{
    /// allocate a new VariadicCountedHashSet with a specific hasher
    pub fn with_hasher(hasher: S) -> Self {
        Self {
            table: HashTable::new(),
            hasher,
            len: 0,
        }
    }
    /// allocate a new VariadicCountedHashSet with a specific hasher and capacity
    pub fn with_capacity_and_hasher(capacity: usize, hasher: S) -> Self {
        Self {
            table: HashTable::with_capacity(capacity),
            hasher,
            len: 0,
        }
    }
}

// THIS CODE ADAPTED FROM hashbrown::HashTable
impl<K, S> Extend<K> for VariadicCountedHashSet<K, S>
where
    K: Eq + Hash + PartialEqVariadic + Clone,
    S: BuildHasher,
    for<'a> K::AsRefVar<'a>: Hash,
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
        // TODO: get reserve to work here
        // let hasher = self.hasher.build_hasher();
        // self.table.reserve(reserve, hasher);
        iter.for_each(move |key| {
            // TODO: super inefficient. Need a insert_with_count method
            self.insert(key);
        });
    }
}

impl<T, S> PartialEq for VariadicCountedHashSet<T, S>
where
    T: Eq + Hash + PartialEqVariadic + Clone,
    S: BuildHasher,
    for<'a> T::AsRefVar<'a>: Hash,
{
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }

        // let v: Vec<&(T, usize)> =
        self.table.iter().all(|(key, count)| {
            if let Some((_, match_val)) = other.get(key.as_ref_var()) {
                match_val == count
            } else {
                false
            }
        })
    }
}

impl<T, S> FromIterator<T> for VariadicCountedHashSet<T, S>
where
    T: Eq + Hash + PartialEqVariadic + Clone,
    S: BuildHasher + Default,
    for<'a> T::AsRefVar<'a>: Hash,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::with_hasher(Default::default());
        set.extend(iter);
        set
    }
}

/// Column storage for Variadic tuples of type Schema
/// An alternative to VariadicHashSet
pub struct VariadicColumnMultiset<Schema>
where
    Schema: VariadicExt,
{
    columns: Schema::IntoVec,
    last_offset: usize,
}

impl<T> VariadicColumnMultiset<T>
where
    T: VariadicExt,
{
    /// initialize an empty columnar set
    pub fn new() -> Self {
        Self {
            columns: <T::IntoVec as Default>::default(),
            last_offset: 0,
        }
    }
}

impl<T> Default for VariadicColumnMultiset<T>
where
    T: VariadicExt,
{
    fn default() -> Self {
        Self::new()
    }
}

#[sealed]
impl<Schema> VariadicMultiset for VariadicColumnMultiset<Schema>
where
    Schema: PartialEqVariadic,
{
    type Schema = Schema;

    /// Insert an element into the set
    fn insert(&mut self, element: Schema) {
        if self.last_offset == 0 {
            self.columns = element.into_singleton_vec()
        } else {
            self.columns.push(element);
        }
        self.last_offset += 1;
    }

    /// Iterate over the elements of the set
    fn iter(&self) -> impl Iterator<Item = <Schema as VariadicExt>::AsRefVar<'_>> {
        self.columns.zip_vecs()
    }

    /// Return number of elements in the set
    fn len(&self) -> usize {
        self.last_offset
    }

    /// Return true if empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// iterate and drain items from the set
    fn drain(&mut self) -> impl Iterator<Item = Self::Schema> {
        self.last_offset = 0;
        self.columns.drain(0..)
    }

    fn contains(&self, value: <Self::Schema as VariadicExt>::AsRefVar<'_>) -> bool {
        self.iter()
            .any(|t| <Schema as PartialEqVariadic>::eq_ref(t, value))
    }
}

impl<T> fmt::Debug for VariadicColumnMultiset<T>
where
    T: fmt::Debug + VariadicExt + PartialEqVariadic,
    for<'a> T::AsRefVar<'a>: Hash + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<Schema> IntoIterator for VariadicColumnMultiset<Schema>
where
    Schema: PartialEqVariadic,
{
    type Item = Schema;
    type IntoIter = <Schema::IntoVec as VecVariadic>::IntoZip;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.columns.into_zip()
    }
}

impl<K> Extend<K> for VariadicColumnMultiset<K>
where
    K: Eq + Hash + PartialEqVariadic,
    for<'a> K::AsRefVar<'a>: Hash,
{
    // #[cfg_attr(feature = "inline-more", inline)]
    fn extend<T: IntoIterator<Item = K>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        // self.table.reserve(reserve, hasher);
        iter.for_each(move |k| {
            self.insert(k);
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{var_expr, var_type};

    type TestSchema = var_type!(i16, i32, i64, &'static str);

    #[test]
    fn test_collections() {
        let test_data: Vec<TestSchema> = vec![
            var_expr!(1, 1, 1, "hello"),
            var_expr!(1, 1, 1, "hello"),
            var_expr!(1, 1, 1, "world"),
            var_expr!(1, 1, 2, "world"),
        ];

        let mut hash_set: VariadicHashSet<TestSchema> = Default::default();
        hash_set.extend(test_data.clone());
        let mut multi_set: VariadicCountedHashSet<TestSchema> = Default::default();
        multi_set.extend(test_data.clone());
        let mut columnar: VariadicColumnMultiset<TestSchema> = Default::default();
        columnar.extend(test_data.clone());

        assert_eq!(multi_set.len(), 4);
        assert_eq!(columnar.len(), 4);
        assert_eq!(hash_set.len(), 3);

        hash_set.insert(var_expr!(1, 1, 1, "hello"));
        hash_set.insert(var_expr!(2, 1, 1, "dup"));
        hash_set.insert(var_expr!(2, 1, 1, "dup"));
        multi_set.insert(var_expr!(1, 1, 1, "hello"));
        multi_set.insert(var_expr!(2, 1, 1, "dup"));
        multi_set.insert(var_expr!(2, 1, 1, "dup"));
        columnar.insert(var_expr!(1, 1, 1, "hello"));
        columnar.insert(var_expr!(2, 1, 1, "dup"));
        columnar.insert(var_expr!(2, 1, 1, "dup"));

        assert_eq!(multi_set.len(), 7);
        assert_eq!(columnar.len(), 7);
        assert_eq!(hash_set.len(), 4);

        assert!(test_data.iter().all(|t| hash_set.contains(t.as_ref_var())));
        // hash value of get is 16254353334811099159 16396044455507064773
        let bug = multi_set.contains(var_expr!(1, 1, 1, "world").as_ref_var());
        multi_set.insert(var_expr!(1, 1, 1, "world"));
        println!("{}", bug);
        println!("multiset: {:?}", multi_set);
        println!(
            "{}",
            test_data.iter().all(|t| multi_set.contains(t.as_ref_var()))
        );
        assert!(test_data.iter().all(|t| columnar.contains(t.as_ref_var())));

        // multi_set
        //     .clone()
        //     .into_iter()
        //     .for_each(|t| println!("row: {:?}", t));
        // println!("multiset: {:?}", multi_set);
        // columnar
        //     .into_iter()
        //     .for_each(|t| println!("columns: {:?}", t));
    }
}
