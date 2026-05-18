//! [`AccumState`] implementations for common accumulator patterns.
//!
//! Each struct here encapsulates the accumulation logic and drain behavior
//! for a specific operator (fold, reduce, sort), in both owned and borrowed modes.

use core::borrow::BorrowMut;
use core::iter::Once;
use core::marker::PhantomData;

use super::accumulate::AccumState;

// ============================================================================
// Fold (unified: owned T or borrowed &'a mut T via BorrowMut)
// ============================================================================

/// Accumulator state for fold, supporting both owned and borrowed modes
/// via [`BorrowMut`].
///
/// - **Owned mode** (`Accum = T`): accumulates into an owned `T`, emits `T` downstream.
/// - **Borrowed mode** (`Accum = &'a mut T`): accumulates into a reference,
///   emits `&'a mut T` downstream. The value persists across ticks.
///
/// `AccumInner` is the type you actually fold into (what `BorrowMut` resolves to).
pub struct FoldState<Accum, F, AccumInner, Item> {
    /// The accumulator — either an owned value or a mutable reference.
    pub accum: Accum,
    /// The combining function: `(&mut AccumInner, Item) -> ()`.
    pub comb_fn: F,
    /// Marker for `AccumInner` and `Item`.
    pub _phantom: PhantomData<fn(&mut AccumInner, Item)>,
}

impl<Accum, F, AccumInner, Item> FoldState<Accum, F, AccumInner, Item> {
    /// Creates a new `FoldState` with the given accumulator and combining function.
    pub const fn new(accum: Accum, comb_fn: F) -> Self {
        Self {
            accum,
            comb_fn,
            _phantom: PhantomData,
        }
    }
}

impl<Accum, F, AccumInner, Item> AccumState for FoldState<Accum, F, AccumInner, Item>
where
    Accum: BorrowMut<AccumInner>,
    F: FnMut(&mut AccumInner, Item),
{
    type Input = Item;
    type Output = Accum;
    type Iter = Once<Accum>;

    fn accumulate(&mut self, item: Item) {
        (self.comb_fn)(self.accum.borrow_mut(), item);
    }

    fn into_iter(self) -> Self::Iter {
        core::iter::once(self.accum)
    }

    fn size_hint(&self, _input_hint: (usize, Option<usize>)) -> (usize, Option<usize>) {
        (1, Some(1))
    }
}

// ============================================================================
// Reduce (unified: owned Option<T> or borrowed &'a mut Option<T> via BorrowMut)
// ============================================================================

/// Accumulator state for reduce, supporting both owned and borrowed modes
/// via [`BorrowMut`].
///
/// - **Owned mode** (`Accum = Option<T>`): the first item initializes the
///   accumulator, subsequent items are merged. Emits `Option<T>` downstream
///   (codegen can filter_map to unwrap).
/// - **Borrowed mode** (`Accum = &'a mut Option<T>`): same logic, but the
///   option persists across ticks. Emits `&'a mut Option<T>` downstream
///   (codegen can map to `.as_mut()` to get `Option<&'a mut T>`).
///
/// In both cases, the downstream receives the `Accum` itself (mirroring fold).
/// The "no items → no output" filtering is handled in codegen via a downstream
/// `filter_map`.
pub struct ReduceState<Accum, F, T> {
    /// The accumulator — either `Option<T>` or `&'a mut Option<T>`.
    pub accum: Accum,
    /// The reduce function: `(&mut T, T) -> ()`.
    pub reduce_fn: F,
    /// Marker for the item type.
    pub _phantom: PhantomData<fn(&mut T, T)>,
}

impl<Accum, F, T> ReduceState<Accum, F, T> {
    /// Creates a new `ReduceState` with the given accumulator and reduce function.
    pub const fn new(accum: Accum, reduce_fn: F) -> Self {
        Self {
            accum,
            reduce_fn,
            _phantom: PhantomData,
        }
    }
}

impl<Accum, F, T> AccumState for ReduceState<Accum, F, T>
where
    Accum: BorrowMut<Option<T>>,
    F: FnMut(&mut T, T),
{
    type Input = T;
    type Output = Accum;
    type Iter = Once<Accum>;

    fn accumulate(&mut self, item: T) {
        match self.accum.borrow_mut() {
            Some(acc) => (self.reduce_fn)(acc, item),
            None => *self.accum.borrow_mut() = Some(item),
        }
    }

    fn into_iter(self) -> Self::Iter {
        core::iter::once(self.accum)
    }

    fn size_hint(&self, _input_hint: (usize, Option<usize>)) -> (usize, Option<usize>) {
        (1, Some(1))
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

impl<T> Default for SortState<T> {
    fn default() -> Self {
        Self::new()
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

    fn size_hint(&self, input_hint: (usize, Option<usize>)) -> (usize, Option<usize>) {
        // Sort preserves cardinality: output count = current buffer + remaining input.
        let buffered = self.buf.len();
        let lower = buffered + input_hint.0;
        let upper = input_hint.1.map(|u| buffered + u);
        (lower, upper)
    }
}

#[cfg(feature = "alloc")]
extern crate alloc;
