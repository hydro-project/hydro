//! Tick-scoped slab allocator with stable slots for DFIR handoff buffers.
//!
//! Each handoff has a stable ID (0..N). The slab remembers each slot's region
//! across ticks — no re-allocation after warmup. Between ticks, `reset()` can
//! compact/defragment since no live pointers exist.

use std::alloc::{Layout, alloc, dealloc, realloc};
use std::ptr;

/// A slab allocator with N stable slots, backed by a single contiguous buffer.
///
/// Slots are packed contiguously. Each slot's available capacity is implicitly
/// the distance to the next slot (or buffer end for the last slot).
pub struct TickSlab<const N: usize> {
    buf: *mut u8,
    buf_cap: usize,
    /// `slot_order[position]` = handoff ID at that position in the buffer.
    slot_order: [usize; N],
    /// `slot_pos[handoff_id]` = position in the buffer (index into slot_order/offsets).
    slot_pos: [usize; N],
    /// `offsets[position]` = byte offset where that position's region starts.
    offsets: [usize; N],
    /// `requested_cap[handoff_id]` = bytes actually needed (high-water from last tick).
    requested_cap: [usize; N],
    /// Number of slots that have been initialized (first use).
    initialized: usize,
}

impl<const N: usize> TickSlab<N> {
    /// Create a new slab with no backing memory. Allocates on first use.
    pub fn new() -> Self {
        Self {
            buf: ptr::null_mut(),
            buf_cap: 0,
            slot_order: [0; N],
            slot_pos: [0; N],
            offsets: [0; N],
            requested_cap: [0; N],
            initialized: 0,
        }
    }

    /// Get the raw pointer and available capacity (in bytes) for a slot.
    /// Allocates/grows the buffer if needed.
    ///
    /// # Safety
    /// The returned pointer is valid until the next `grow_slot` or `reset` call.
    pub fn slot_ptr_and_cap(&mut self, id: usize) -> (*mut u8, usize) {
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

    /// Ensure slot `id` has at least `min_bytes` of capacity.
    /// If it doesn't, move it to the end of the buffer (growing the buffer if needed).
    pub fn ensure_capacity(&mut self, id: usize, min_bytes: usize) {
        if id >= self.initialized || self.slot_pos[id] >= self.initialized {
            // First time initialization for this slot
            self.init_slot(id, min_bytes);
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
            return; // Already big enough
        }

        // Update requested cap
        self.requested_cap[id] = min_bytes;

        // Need more space. If we're the last slot, just grow the buffer.
        if pos + 1 == self.initialized {
            self.grow_buf(offset + min_bytes);
            return;
        }

        // Not last: move to end. Shift subsequent slots left to fill gap.
        let old_offset = offset;
        let old_cap = current_cap;

        // Actually move the memory for shifted slots
        unsafe {
            let src = self.buf.add(old_offset + old_cap);
            let dst = self.buf.add(old_offset);
            let end = self.end_offset();
            let move_len = end - (old_offset + old_cap);
            if move_len > 0 {
                ptr::copy(src, dst, move_len);
            }
        }

        // Shift positions after `pos` one to the left
        for i in pos..self.initialized - 1 {
            self.slot_order[i] = self.slot_order[i + 1];
            self.offsets[i] = self.offsets[i + 1] - old_cap;
            self.slot_pos[self.slot_order[i]] = i;
        }

        // Place this slot at the end
        let new_pos = self.initialized - 1;
        let new_offset = self.end_offset() - old_cap;
        self.slot_order[new_pos] = id;
        self.slot_pos[id] = new_pos;
        self.offsets[new_pos] = new_offset;

        // Now grow the buffer to accommodate min_bytes
        let needed = new_offset + min_bytes;
        if needed > self.buf_cap {
            self.grow_buf(needed);
        }
    }

    /// Initialize a slot for first use.
    fn init_slot(&mut self, id: usize, min_bytes: usize) {
        let pos = self.initialized;
        let offset = self.end_offset();

        self.slot_order[pos] = id;
        self.slot_pos[id] = pos;
        self.offsets[pos] = offset;
        self.requested_cap[id] = min_bytes;
        self.initialized += 1;

        // Ensure buffer is large enough
        let needed = offset + min_bytes;
        if needed > self.buf_cap {
            self.grow_buf(needed);
        }
    }

    /// Current end of used space.
    fn end_offset(&self) -> usize {
        if self.initialized == 0 {
            0
        } else {
            // End is the last slot's offset + its allocated size
            let last_pos = self.initialized - 1;
            let last_id = self.slot_order[last_pos];
            self.offsets[last_pos] + self.requested_cap[last_id]
        }
    }

    /// Grow the backing buffer to at least `min_cap` bytes.
    fn grow_buf(&mut self, min_cap: usize) {
        if min_cap <= self.buf_cap {
            return;
        }
        let new_cap = min_cap.next_power_of_two().max(64);
        unsafe {
            if self.buf.is_null() {
                let layout = Layout::from_size_align_unchecked(new_cap, 16);
                self.buf = alloc(layout);
            } else {
                let old_layout = Layout::from_size_align_unchecked(self.buf_cap, 16);
                self.buf = realloc(self.buf, old_layout, new_cap);
            }
        }
        self.buf_cap = new_cap;
    }

    /// Update the requested capacity for a slot (called at end of tick).
    pub fn set_requested_cap(&mut self, id: usize, cap_bytes: usize) {
        self.requested_cap[id] = cap_bytes;
    }

    /// Reset for a new tick. Compacts if beneficial.
    pub fn reset(&mut self) {
        // For now, just update offsets based on requested_caps (simple compaction).
        // This packs slots tightly at their requested sizes.
        if self.initialized == 0 {
            return;
        }
        let mut offset = 0;
        for pos in 0..self.initialized {
            self.offsets[pos] = offset;
            let id = self.slot_order[pos];
            offset += self.requested_cap[id];
        }
        // Grow buffer if compacted layout exceeds current capacity
        if offset > self.buf_cap {
            self.grow_buf(offset);
        }
    }
}

impl<const N: usize> Drop for TickSlab<N> {
    fn drop(&mut self) {
        if !self.buf.is_null() && self.buf_cap > 0 {
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.buf_cap, 16);
                dealloc(self.buf, layout);
            }
        }
    }
}

/// A Vec-like handle into a [`TickSlab`] slot.
///
/// Tick-local: created each tick from a slot, can hold any `T` including references.
/// Does NOT hold a reference to the slab — capacity is fixed at creation time.
/// Use `reserve` on the slab before creating, or accept that push may panic on overflow.
pub struct SlabVec<'a, T> {
    ptr: *mut T,
    len: usize,
    cap: usize, // in elements
    _marker: std::marker::PhantomData<&'a mut [T]>,
}

impl<'a, T> SlabVec<'a, T> {
    /// Create a SlabVec from a raw pointer and capacity.
    ///
    /// # Safety
    /// `ptr` must be valid for `cap` elements of `T`, properly aligned, and
    /// not aliased for the lifetime `'a`.
    pub unsafe fn from_raw(ptr: *mut T, cap: usize) -> Self {
        Self {
            ptr,
            len: 0,
            cap,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn push(&mut self, item: T) {
        assert!(self.len < self.cap, "SlabVec overflow: len={}, cap={}", self.len, self.cap);
        unsafe {
            ptr::write(self.ptr.add(self.len), item);
        }
        self.len += 1;
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
            _marker: std::marker::PhantomData,
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.len {
            unsafe { ptr::drop_in_place(self.ptr.add(i)); }
        }
        self.len = 0;
    }
}

impl<'a, T> Drop for SlabVec<'a, T> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Drain iterator for [`SlabVec`].
pub struct SlabVecDrain<'a, T> {
    ptr: *mut T,
    idx: usize,
    len: usize,
    _marker: std::marker::PhantomData<&'a mut T>,
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
        // Drop remaining undrained elements
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
        let mut slab = TickSlab::<4>::new();
        slab.ensure_capacity(0, 64);

        unsafe {
            let (ptr, cap) = slab.slot_ptr_and_cap(0);
            let mut v0: SlabVec<i32> = SlabVec::from_raw(ptr as *mut i32, cap / 4);
            v0.push(1);
            v0.push(2);
            v0.push(3);
            assert_eq!(v0.len(), 3);
            let items: Vec<i32> = v0.drain().collect();
            assert_eq!(items, vec![1, 2, 3]);
        }
    }

    #[test]
    fn multiple_slots() {
        let mut slab = TickSlab::<4>::new();
        slab.ensure_capacity(0, 32);
        slab.ensure_capacity(1, 64);

        unsafe {
            let (ptr0, cap0) = slab.slot_ptr_and_cap(0);
            let (ptr1, cap1) = slab.slot_ptr_and_cap(1);

            let mut v0: SlabVec<i32> = SlabVec::from_raw(ptr0 as *mut i32, cap0 / 4);
            let mut v1: SlabVec<i32> = SlabVec::from_raw(ptr1 as *mut i32, cap1 / 4);

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
    fn across_ticks() {
        let mut slab = TickSlab::<2>::new();

        // Tick 1: grow slot 0
        slab.ensure_capacity(0, 400); // 100 i32s
        slab.set_requested_cap(0, 400);

        slab.reset();

        // Tick 2: slot 0 should have capacity from tick 1
        let (ptr, cap) = slab.slot_ptr_and_cap(0);
        assert!(cap >= 400);

        unsafe {
            let mut v0: SlabVec<i32> = SlabVec::from_raw(ptr as *mut i32, cap / 4);
            for i in 0..100 {
                v0.push(i);
            }
            assert_eq!(v0.len(), 100);
        }
    }
}
