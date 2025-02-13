//! Module for [`MonotonicMap`].

use super::clear::Clear;

/// A map-like interface which in reality only stores one value at a time.
///
/// The keys must be monotonically increasing (i.e. timestamps). For DFIR, this allows state
/// to be stored which resets each tick by using the tick counter as the key. In the generic `Map`
/// case it can be swapped out for a true map to allow processing of multiple ticks of data at
/// once.
#[derive(Clone, Debug)]
pub struct MonotonicMap<K, V>
where
    K: PartialOrd,
{
    key: Option<K>,
    val: V,
}

impl<K, V> Default for MonotonicMap<K, V>
where
    K: PartialOrd,
    V: Default,
{
    fn default() -> Self {
        Self {
            key: None,
            val: Default::default(),
        }
    }
}

impl<K, V> MonotonicMap<K, V>
where
    K: PartialOrd,
{
    /// Creates a new `MonotonicMap` initialized with the given value. The
    /// vaue will be `Clear`ed before it is accessed.
    pub fn new_init(val: V) -> Self {
        Self { key: None, val }
    }

    /// Inserts the value using the function if new `key` is strictly later than the current key.
    pub fn get_mut_with(&mut self, key: K, init: impl FnOnce() -> V) -> &mut V {
        if self.key.as_ref().is_none_or(|old_key| old_key < &key) {
            self.key = Some(key);
            self.val = (init)();
        }
        &mut self.val
    }

    // /// Returns the value for the monotonically increasing key, or `None` if
    // /// the key has already passed.
    // pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
    //     if self
    //         .key
    //         .as_ref()
    //         .map_or(true, |old_key| old_key <= &key)
    //     {
    //         self.key = Some(key);
    //         Some(&mut self.val)
    //     } else {
    //         None
    //     }
    // }
}

impl<K, V> MonotonicMap<K, V>
where
    K: PartialOrd,
    V: Default,
{
    /// Gets a mutable reference to the inner value. If `key` is strictly later than the existing
    /// key, the value will be reset to `Default::default`.
    pub fn get_mut_default(&mut self, key: K) -> &mut V {
        self.get_mut_with(key, Default::default)
    }
}

impl<K, V> MonotonicMap<K, V>
where
    K: PartialOrd,
    V: Clear,
{
    /// Gets a mutable reference to the inner value. If `key` is strictly later than the existing
    /// key, the value will be cleared via the [`Clear`] trait.
    pub fn get_mut_clear(&mut self, key: K) -> &mut V {
        if self.key.as_ref().is_none_or(|old_key| old_key < &key) {
            self.key = Some(key);
            self.val.clear();
        }
        &mut self.val
    }
}
