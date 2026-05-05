//! [`AccumState`] implementations for common accumulator patterns.
//!
//! Each struct here encapsulates the accumulation logic and drain behavior
//! for a specific operator (fold, reduce, sort), in both owned and borrowed modes.

use core::iter::Once;
use core::marker::PhantomData;

use super::accumulate::AccumState;

// ============================================================================
// Fold (owned mode)
// ============================================================================

/// Accumulator state for fold in owned / `'tick` mode.
///
/// Accumulates items into an owned value `T` using a combining function,
/// then emits the final value as a single-element iterator.
pub struct FoldState<T, F, Item> {
    /// The accumulator value.
    pub val: T,
    /// The combining function: `(&mut T, Item) -> ()`.
    pub comb_fn: F,
    /// Marker for the input item type.
    pub _phantom: PhantomData<fn(Item)>,
}

impl<T, F, Item> FoldState<T, F, Item> {
    /// Creates a new `FoldState` with the given initial value and combining function.
    pub const fn new(val: T, comb_fn: F) -> Self {
        Self {
            val,
            comb_fn,
            _phantom: PhantomData,
        }
    }
}

impl<T, F, Item> AccumState for FoldState<T, F, Item>
where
    F: FnMut(&mut T, Item),
{
    type Input = Item;
    type Output = T;
    type Iter = Once<T>;

    fn accumulate(&mut self, item: Item) {
        (self.comb_fn)(&mut self.val, item);
    }

    fn into_iter(self) -> Self::Iter {
        core::iter::once(self.val)
    }
}

// ============================================================================
// Fold (borrowed mode)
// ============================================================================

/// Accumulator state for fold in borrowed / `'static` mode.
///
/// Accumulates items into a `&'a mut T` reference that outlives the tick,
/// then emits `&'a mut T` downstream. The value persists across ticks.
pub struct FoldBorrowed<'a, T, F, Item> {
    /// Mutable reference to externally-owned accumulator.
    pub val: &'a mut T,
    /// The combining function: `(&mut T, Item) -> ()`.
    pub comb_fn: F,
    /// Marker for the input item type.
    pub _phantom: PhantomData<fn(Item)>,
}

impl<'a, T, F, Item> FoldBorrowed<'a, T, F, Item> {
    /// Creates a new `FoldBorrowed` with the given reference and combining function.
    pub fn new(val: &'a mut T, comb_fn: F) -> Self {
        Self {
            val,
            comb_fn,
            _phantom: PhantomData,
        }
    }
}

impl<'a, T, F, Item> AccumState for FoldBorrowed<'a, T, F, Item>
where
    F: FnMut(&mut T, Item),
{
    type Input = Item;
    type Output = &'a mut T;
    type Iter = Once<&'a mut T>;

    fn accumulate(&mut self, item: Item) {
        (self.comb_fn)(self.val, item);
    }

    fn into_iter(self) -> Self::Iter {
        core::iter::once(self.val)
    }
}

// ============================================================================
// Reduce (owned mode)
// ============================================================================

/// Accumulator state for reduce in owned / `'tick` mode.
///
/// Reduces items into `Option<T>`: the first item initializes the accumulator,
/// subsequent items are merged via the reduce function. Emits 0 or 1 items.
pub struct ReduceState<T, F> {
    /// The accumulator, `None` until the first item arrives.
    pub val: Option<T>,
    /// The reduce function: `(&mut T, T) -> ()`.
    pub reduce_fn: F,
}

impl<T, F> ReduceState<T, F> {
    /// Creates a new `ReduceState` with no initial value.
    pub const fn new(reduce_fn: F) -> Self {
        Self {
            val: None,
            reduce_fn,
        }
    }

    /// Creates a new `ReduceState` with an optional initial value.
    pub const fn with_initial(val: Option<T>, reduce_fn: F) -> Self {
        Self { val, reduce_fn }
    }
}

impl<T, F> AccumState for ReduceState<T, F>
where
    F: FnMut(&mut T, T),
{
    type Input = T;
    type Output = T;
    type Iter = core::option::IntoIter<T>;

    fn accumulate(&mut self, item: T) {
        match &mut self.val {
            Some(acc) => (self.reduce_fn)(acc, item),
            None => self.val = Some(item),
        }
    }

    fn into_iter(self) -> Self::Iter {
        self.val.into_iter()
    }
}

// ============================================================================
// Reduce (borrowed mode)
// ============================================================================

/// Accumulator state for reduce in borrowed / `'static` mode.
///
/// Reduces items into a `&'a mut Option<T>` that persists across ticks.
/// On finalize, emits `&'a mut T` (a reference into the persistent option)
/// if a value is present, or nothing if empty.
pub struct ReduceBorrowed<'a, T, F> {
    /// Mutable reference to externally-owned option storage.
    pub val: &'a mut Option<T>,
    /// The reduce function: `(&mut T, T) -> ()`.
    pub reduce_fn: F,
}

impl<'a, T, F> ReduceBorrowed<'a, T, F> {
    /// Creates a new `ReduceBorrowed` with the given reference and reduce function.
    pub fn new(val: &'a mut Option<T>, reduce_fn: F) -> Self {
        Self { val, reduce_fn }
    }
}

impl<'a, T, F> AccumState for ReduceBorrowed<'a, T, F>
where
    F: FnMut(&mut T, T),
{
    type Input = T;
    type Output = &'a mut T;
    type Iter = core::option::IterMut<'a, T>;

    fn accumulate(&mut self, item: T) {
        match self.val {
            Some(acc) => (self.reduce_fn)(acc, item),
            None => *self.val = Some(item),
        }
    }

    fn into_iter(self) -> Self::Iter {
        self.val.iter_mut()
    }
}

// ============================================================================
// Sort (owned mode, requires alloc)
// ============================================================================

/// Accumulator state for sort.
///
/// Collects all items into a `Vec`, sorts them on finalize, and emits
/// them in sorted order.
#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
pub struct SortState<T> {
    /// Buffer for collected items.
    pub buf: alloc::vec::Vec<T>,
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
impl<T> SortState<T> {
    /// Creates a new empty `SortState`.
    pub const fn new() -> Self {
        Self {
            buf: alloc::vec::Vec::new(),
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(docsrs, doc(cfg(feature = "alloc")))]
impl<T: Ord> AccumState for SortState<T> {
    type Input = T;
    type Output = T;
    type Iter = alloc::vec::IntoIter<T>;

    fn accumulate(&mut self, item: T) {
        self.buf.push(item);
    }

    fn into_iter(self) -> Self::Iter {
        let mut buf = self.buf;
        buf.sort_unstable();
        buf.into_iter()
    }
}

#[cfg(feature = "alloc")]
extern crate alloc;
