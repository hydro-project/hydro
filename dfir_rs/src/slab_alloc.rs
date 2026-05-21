//! Tick-scoped slab allocator with stable slots for DFIR handoff buffers.
//!
//! Each handoff has a stable ID (0..N). The slab remembers each slot's region
//! across ticks — no re-allocation after warmup. Between ticks, `reset()` can
//! compact/defragment since no live pointers exist.

use std::alloc::{Layout, alloc, dealloc, realloc};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ptr;

/// Inner mutable state of the slab.
struct TickSlabInner<const N: usize> {
    buf: *mut u8,
    buf_cap: usize,
    /// `slot_order[position]` = handoff ID at that position in the buffer.
    slot_order: [usize; N],
    /// `slot_pos[handoff_id]` = position in the buffer.
    slot_pos: [usize; N],
    /// `offsets[position]` = byte offset where that position's region starts.
    offsets: [usize; N],
    /// `requested_cap[handoff_id]` = bytes actually needed (high-water).
    requested_cap: [usize; N],
    /// Number of slots that have been initialized.
    initialized: usize,
}

/// A slab allocator with N stable slots, backed by a single contiguous buffer.
///
/// Uses `RefCell` for interior mutability so multiple `SlabVec`s can coexist
/// while still allowing growth via `reserve`.
pub struct TickSlab<const N: usize> {
    inner: RefCell<TickSlabInner<N>>,
}

impl<const N: usize> TickSlab<N> {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(TickSlabInner {
                buf: ptr::null_mut(),
                buf_cap: 0,
                slot_order: [0; N],
                slot_pos: [0; N],
                offsets: [0; N],
                requested_cap: [0; N],
                initialized: 0,
            }),
        }
    }

    /// Get a [`SlabVec`] for the given slot, ensuring at least `min_cap` elements fit.
    ///
    /// # Safety
    /// - Must not create two `SlabVec`s for the same slot ID simultaneously.
    /// - `T` must be consistent for a given slot ID across calls.
    pub unsafe fn vec<T>(&self, id: usize, min_cap: usize) -> SlabVec<'_, T, N> {
        let align = std::mem::align_of::<T>();
        let elem_size = std::mem::size_of::<T>().max(1);
        let min_bytes = min_cap * elem_size;

        {
            let mut inner = self.inner.borrow_mut();
            inner.ensure_capacity(id, min_bytes, align);
        }

        let inner = self.inner.borrow();
        let (ptr, cap_bytes) = inner.slot_ptr_and_cap(id);
        debug_assert!(
            ptr as usize % align == 0,
            "slot pointer not aligned for T"
        );
        let cap_elems = cap_bytes / elem_size;

        SlabVec {
            ptr: ptr as *mut T,
            len: 0,
            cap: cap_elems,
            slot_id: id,
            slab: self,
            _marker: PhantomData,
        }
    }

    /// Reset for a new tick. Compacts slots based on requested_cap.
    ///
    /// Must not be called while any `SlabVec` is live.
    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();
        if inner.initialized == 0 {
            return;
        }
        let mut offset = 0;
        for pos in 0..inner.initialized {
            inner.offsets[pos] = offset;
            let id = inner.slot_order[pos];
            offset += inner.requested_cap[id];
        }
        if offset > inner.buf_cap {
            inner.grow_buf(offset);
        }
    }
}

impl<const N: usize> Drop for TickSlab<N> {
    fn drop(&mut self) {
        let inner = self.inner.get_mut();
        if !inner.buf.is_null() && inner.buf_cap > 0 {
            unsafe {
                let layout = Layout::from_size_align(inner.buf_cap, 16).unwrap();
                dealloc(inner.buf, layout);
            }
        }
    }
}

impl<const N: usize> TickSlabInner<N> {
    fn slot_ptr_and_cap(&self, id: usize) -> (*mut u8, usize) {
        debug_assert!(id < N);
        let pos = self.slot_pos[id];
        let offset = self.offsets[pos];
        let cap = if pos + 1 < self.initialized {
            self.offsets[pos + 1] - offset
        } else {
            self.buf_cap - offset
        };
        (unsafe { self.buf.add(offset) }, cap)
    }

    fn ensure_capacity(&mut self, id: usize, min_bytes: usize, align: usize) {
        if id >= N {
            panic!("slot id {} exceeds slab capacity {}", id, N);
        }

        // Check if this slot has been initialized
        let is_new = if self.initialized == 0 {
            true
        } else {
            // A slot is "new" if its position hasn't been set yet.
            // We detect this by checking if the slot_order at slot_pos[id] == id.
            let pos = self.slot_pos[id];
            pos >= self.initialized || self.slot_order[pos] != id
        };

        if is_new {
            self.init_slot(id, min_bytes, align);
            return;
        }

        let pos = self.slot_pos[id];
        let offset = self.offsets[pos];
        let current_cap = if pos + 1 < self.initialized {
            self.offsets[pos + 1] - offset
        } else {
            self.buf_cap - offset
        };

        if current_cap >= min_bytes {
            return;
        }

        self.requested_cap[id] = min_bytes;

        // Last slot: just grow the buffer
        if pos + 1 == self.initialized {
            self.grow_buf(offset + min_bytes);
            return;
        }

        // Not last: move to end
        let old_offset = offset;
        let old_cap = current_cap;

        // Move memory for shifted slots
        let end = self.end_offset();
        let move_len = end - (old_offset + old_cap);
        if move_len > 0 {
            unsafe {
                let src = self.buf.add(old_offset + old_cap);
                let dst = self.buf.add(old_offset);
                ptr::copy(src, dst, move_len);
            }
        }

        // Shift positions left
        for i in pos..self.initialized - 1 {
            self.slot_order[i] = self.slot_order[i + 1];
            self.offsets[i] = self.offsets[i + 1] - old_cap;
            self.slot_pos[self.slot_order[i]] = i;
        }

        // Place at end, aligned
        let new_pos = self.initialized - 1;
        let raw_offset = if new_pos > 0 {
            let prev_id = self.slot_order[new_pos - 1];
            self.offsets[new_pos - 1] + self.requested_cap[prev_id]
        } else {
            0
        };
        let new_offset = (raw_offset + align - 1) & !(align - 1);
        self.slot_order[new_pos] = id;
        self.slot_pos[id] = new_pos;
        self.offsets[new_pos] = new_offset;

        let needed = new_offset + min_bytes;
        if needed > self.buf_cap {
            self.grow_buf(needed);
        }
    }

    fn init_slot(&mut self, id: usize, min_bytes: usize, align: usize) {
        let pos = self.initialized;
        let raw_offset = self.end_offset();
        let offset = (raw_offset + align - 1) & !(align - 1);

        self.slot_order[pos] = id;
        self.slot_pos[id] = pos;
        self.offsets[pos] = offset;
        self.requested_cap[id] = min_bytes;
        self.initialized += 1;

        let needed = offset + min_bytes;
        if needed > self.buf_cap {
            self.grow_buf(needed);
        }
    }

    fn end_offset(&self) -> usize {
        if self.initialized == 0 {
            0
        } else {
            let last_pos = self.initialized - 1;
            let last_id = self.slot_order[last_pos];
            self.offsets[last_pos] + self.requested_cap[last_id]
        }
    }

    fn grow_buf(&mut self, min_cap: usize) {
        if min_cap <= self.buf_cap {
            return;
        }
        let new_cap = min_cap.next_power_of_two().max(64);
        unsafe {
            if self.buf.is_null() {
                let layout = Layout::from_size_align(new_cap, 16).unwrap();
                self.buf = alloc(layout);
            } else {
                let old_layout = Layout::from_size_align(self.buf_cap, 16).unwrap();
                self.buf = realloc(self.buf, old_layout, new_cap);
            }
        }
        self.buf_cap = new_cap;
    }
}

/// A Vec-like handle into a [`TickSlab`] slot.
///
/// Tick-local: can hold any `T` including references.
/// Holds a shared reference to the slab for `reserve`/growth.
pub struct SlabVec<'a, T, const N: usize> {
    pub(crate) ptr: *mut T,
    len: usize,
    cap: usize,
    slot_id: usize,
    slab: &'a TickSlab<N>,
    _marker: PhantomData<&'a mut [T]>,
}

impl<'a, T, const N: usize> SlabVec<'a, T, N> {
    pub fn push(&mut self, item: T) {
        if self.len == self.cap {
            self.grow(1);
        }
        unsafe {
            ptr::write(self.ptr.add(self.len), item);
        }
        self.len += 1;
    }

    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len + additional;
        if needed > self.cap {
            self.grow(additional);
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn drain(&mut self) -> SlabVecDrain<'_, T> {
        let len = self.len;
        self.len = 0;
        SlabVecDrain {
            ptr: self.ptr,
            idx: 0,
            len,
            _marker: PhantomData,
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.len {
            unsafe { ptr::drop_in_place(self.ptr.add(i)); }
        }
        self.len = 0;
    }

    fn grow(&mut self, additional: usize) {
        let needed = self.len + additional;
        let new_cap = needed.max(self.cap * 2).max(4);
        let elem_size = std::mem::size_of::<T>().max(1);
        let align = std::mem::align_of::<T>();
        let new_bytes = new_cap * elem_size;

        {
            let mut inner = self.slab.inner.borrow_mut();
            inner.ensure_capacity(self.slot_id, new_bytes, align);
            let (ptr, cap_bytes) = inner.slot_ptr_and_cap(self.slot_id);
            self.ptr = ptr as *mut T;
            self.cap = cap_bytes / elem_size;
        }
    }
}

impl<'a, T, const N: usize> Drop for SlabVec<'a, T, N> {
    fn drop(&mut self) {
        // Drop remaining elements
        for i in 0..self.len {
            unsafe { ptr::drop_in_place(self.ptr.add(i)); }
        }
        // Update requested_cap for next tick
        let elem_size = std::mem::size_of::<T>().max(1);
        let used_bytes = self.cap * elem_size;
        self.slab.inner.borrow_mut().requested_cap[self.slot_id] = used_bytes;
    }
}

/// Drain iterator for [`SlabVec`].
pub struct SlabVecDrain<'a, T> {
    ptr: *mut T,
    idx: usize,
    len: usize,
    _marker: PhantomData<&'a mut [T]>,
}

impl<'a, T> Iterator for SlabVecDrain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.idx < self.len {
            let item = unsafe { ptr::read(self.ptr.add(self.idx)) };
            self.idx += 1;
            Some(item)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len - self.idx;
        (remaining, Some(remaining))
    }
}

impl<'a, T> Drop for SlabVecDrain<'a, T> {
    fn drop(&mut self) {
        for i in self.idx..self.len {
            unsafe { ptr::drop_in_place(self.ptr.add(i)); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_usage() {
        let slab = TickSlab::<4>::new();
        unsafe {
            let mut v: SlabVec<i32, 4> = slab.vec(0, 16);
            v.push(1);
            v.push(2);
            v.push(3);
            assert_eq!(v.len(), 3);
            let items: Vec<i32> = v.drain().collect();
            assert_eq!(items, vec![1, 2, 3]);
        }
    }

    #[test]
    fn multiple_slots_coexist() {
        let slab = TickSlab::<4>::new();
        unsafe {
            let mut v0: SlabVec<i32, 4> = slab.vec(0, 8);
            let mut v1: SlabVec<i32, 4> = slab.vec(1, 8);

            v0.push(10);
            v0.push(20);
            v1.push(30);
            v1.push(40);
            v1.push(50);

            let items0: Vec<i32> = v0.drain().collect();
            let items1: Vec<i32> = v1.drain().collect();
            assert_eq!(items0, vec![10, 20]);
            assert_eq!(items1, vec![30, 40, 50]);
        }
    }

    #[test]
    fn grow_via_push() {
        let slab = TickSlab::<2>::new();
        unsafe {
            let mut v: SlabVec<i32, 2> = slab.vec(0, 2); // start small
            for i in 0..100 {
                v.push(i);
            }
            assert_eq!(v.len(), 100);
            let items: Vec<i32> = v.drain().collect();
            assert_eq!(items, (0..100).collect::<Vec<_>>());
        }
    }

    #[test]
    fn across_ticks() {
        let slab = TickSlab::<2>::new();

        // Tick 1
        unsafe {
            let mut v: SlabVec<i32, 2> = slab.vec(0, 4);
            for i in 0..100 {
                v.push(i);
            }
            // Drop updates requested_cap
        }

        slab.reset();

        // Tick 2: capacity retained
        unsafe {
            let v: SlabVec<i32, 2> = slab.vec(0, 0);
            assert!(v.capacity() >= 100);
        }
    }

    #[test]
    fn alignment() {
        let slab = TickSlab::<3>::new();
        unsafe {
            let _v0: SlabVec<u8, 3> = slab.vec(0, 3);
            let v1: SlabVec<u64, 3> = slab.vec(1, 4);
            assert_eq!(v1.ptr as usize % 8, 0, "u64 slot must be 8-byte aligned");
        }
    }
}
