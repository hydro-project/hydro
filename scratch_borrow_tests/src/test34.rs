//! Test 34: Bump arena with pre-allocated slots.
//! Instead of allocating from the arena at runtime, pre-allocate all handoff
//! slots at arena creation time. Each tick just resets the lengths.
//! This avoids the multiple-mutable-borrow problem.
//!
//! Key insight: the proc macro KNOWS how many handoffs exist and their max sizes.
//! So it can pre-allocate all slots upfront. The "arena" is really just a struct
//! of pre-sized buffers — which is what the codegen already produces!
//!
//! The real question becomes: can we have a SINGLE captured allocation
//! (one big [u8; N]) that we slice into multiple tick-local Vecs?
//! RESULT: ✅ PASS

use std::mem::MaybeUninit;

/// A pre-partitioned arena. The proc macro computes the layout at compile time.
/// Each "slot" is a region of the arena assigned to one handoff.
struct PrePartitionedArena {
    // One contiguous allocation, partitioned at known offsets.
    // For no_std: [u8; TOTAL_SIZE] (compile-time constant)
    // For std: Vec<u8> (heap, allocated once)
    buf: Vec<u8>,
}

/// A handle to a pre-assigned region in the arena.
/// Captured across ticks — just stores offset and capacity.
/// The actual typed access happens tick-locally.
struct SlotHandle {
    offset: usize,
    cap: usize,  // in elements
    elem_size: usize,
    elem_align: usize,
}

impl PrePartitionedArena {
    fn new(size: usize) -> Self {
        let mut buf = vec![0u8; size];
        Self { buf }
    }

    /// Get a raw pointer to a slot's memory region.
    fn slot_ptr(&mut self, handle: &SlotHandle) -> *mut u8 {
        unsafe { self.buf.as_mut_ptr().add(handle.offset) }
    }

    /// Allocate a slot for `cap` elements of type T. Returns a handle.
    fn alloc_slot<T>(&mut self, cap: usize, current_offset: &mut usize) -> SlotHandle {
        let align = std::mem::align_of::<T>();
        let size = std::mem::size_of::<T>();
        let aligned = (*current_offset + align - 1) & !(align - 1);
        let end = aligned + size * cap;
        assert!(end <= self.buf.len(), "arena too small");
        let handle = SlotHandle {
            offset: aligned,
            cap,
            elem_size: size,
            elem_align: align,
        };
        *current_offset = end;
        handle
    }
}

/// Tick-local typed view into a slot. Can hold any T including references.
struct SlotVec<'a, T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    _marker: std::marker::PhantomData<&'a mut [T]>,
}

impl<'a, T> SlotVec<'a, T> {
    /// Create from arena + handle. Tick-local — T can contain references.
    fn from_slot(arena: &'a mut PrePartitionedArena, handle: &SlotHandle) -> Self {
        let ptr = arena.slot_ptr(handle) as *mut T;
        Self {
            ptr,
            len: 0,
            cap: handle.cap,
            _marker: std::marker::PhantomData,
        }
    }

    fn push(&mut self, val: T) {
        assert!(self.len < self.cap, "SlotVec full");
        unsafe { std::ptr::write(self.ptr.add(self.len), val); }
        self.len += 1;
    }

    fn iter(&self) -> impl Iterator<Item = &T> {
        (0..self.len).map(move |i| unsafe { &*self.ptr.add(i) })
    }

    fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        let len = self.len;
        self.len = 0;
        (0..len).map(move |i| unsafe { std::ptr::read(self.ptr.add(i)) })
    }
}

impl<'a, T> Drop for SlotVec<'a, T> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe { std::ptr::drop_in_place(self.ptr.add(i)); }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    // Pre-allocate arena with known layout (proc macro would compute this)
    let mut arena = PrePartitionedArena::new(4096);
    let mut offset = 0;
    let stream_slot = arena.alloc_slot::<i64>(16, &mut offset);
    let ref_slot = arena.alloc_slot::<&i64>(16, &mut offset);

    let mut tick = async move |input: &[i64]| -> bool {
        // Create tick-local typed views — these CAN hold references
        // Problem: can't create two SlotVecs from the same &mut arena...
        // Need to use raw pointers or split borrows.
        
        // Workaround: compute raw pointers upfront, then create SlotVecs
        let stream_ptr = arena.slot_ptr(&stream_slot) as *mut i64;
        let ref_ptr = arena.slot_ptr(&ref_slot) as *mut &i64;

        let mut stream_len = 0usize;
        let mut ref_len = 0usize;

        // Stratum 0
        {
            for &val in input {
                fold_state += val;
                persist_buf[persist_len] = val;
                persist_len += 1;
                unsafe { std::ptr::write(stream_ptr.add(stream_len), val); }
                stream_len += 1;
            }
            for i in 0..persist_len {
                unsafe { std::ptr::write(ref_ptr.add(ref_len), &persist_buf[i]); }
                ref_len += 1;
            }
        }

        let fold_ref = &fold_state;

        // Stratum 1
        {
            print!("stream (fold={}): ", fold_ref);
            for i in 0..stream_len {
                let val = unsafe { std::ptr::read(stream_ptr.add(i)) };
                print!("{} ", val);
            }
            println!();
            print!("refs: ");
            for i in 0..ref_len {
                let r = unsafe { std::ptr::read(ref_ptr.add(i)) };
                print!("{} ", r);
            }
            println!();
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
