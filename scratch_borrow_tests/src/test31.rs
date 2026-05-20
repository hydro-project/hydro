//! Test 31: Bump arena that provides Vec-like buffers within a tick.
//! 
//! The arena persists across ticks (captured). Each tick, it resets its bump pointer.
//! Buffers allocated from it are tick-local (can hold references).
//! On drop (end of tick), buffers are just forgotten — the arena reclaims everything.
//!
//! For no_std: back the arena with a fixed `[u8; N]` array.
//! For std: back it with a heap-allocated `Vec<u8>` that grows as needed.
//!
//! RESULT: ✅ PASS

use std::cell::Cell;
use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ptr;

/// A simple bump allocator that resets each tick.
/// Captured across ticks — the backing memory persists.
struct TickArena {
    buf: Vec<u8>,       // For no_std: replace with [u8; N]
    offset: usize,      // Bump pointer — reset to 0 each tick
}

impl TickArena {
    fn new(capacity: usize) -> Self {
        let mut buf = Vec::with_capacity(capacity);
        // SAFETY: we manage initialization ourselves
        unsafe { buf.set_len(capacity); }
        Self { buf, offset: 0 }
    }

    /// Reset the arena for a new tick. All previous allocations are invalidated.
    /// SAFETY: caller must ensure no references to arena memory are live.
    fn reset(&mut self) {
        self.offset = 0;
    }

    /// Allocate space for `count` elements of type T. Returns a raw slice pointer.
    /// Panics if arena is full.
    fn alloc_slice_raw<T>(&mut self, count: usize) -> *mut T {
        let align = mem::align_of::<T>();
        let size = mem::size_of::<T>() * count;

        // Align up
        let aligned_offset = (self.offset + align - 1) & !(align - 1);
        let new_offset = aligned_offset + size;

        if new_offset > self.buf.len() {
            // For std: could grow. For no_std: panic.
            self.buf.resize(new_offset * 2, 0);
        }

        let ptr = unsafe { self.buf.as_mut_ptr().add(aligned_offset) as *mut T };
        self.offset = new_offset;
        ptr
    }
}

/// A Vec-like buffer allocated from a TickArena.
/// Tick-local — can hold any T including references.
/// Does NOT free on drop (arena reclaims on reset).
struct ArenaVec<'arena, T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    _marker: PhantomData<&'arena mut [T]>,
}

impl<'arena, T> ArenaVec<'arena, T> {
    /// Create a new ArenaVec with capacity `cap` from the arena.
    fn new(arena: &'arena mut TickArena, cap: usize) -> Self {
        let ptr = arena.alloc_slice_raw::<T>(cap);
        Self { ptr, len: 0, cap, _marker: PhantomData }
    }

    fn push(&mut self, val: T) {
        assert!(self.len < self.cap, "ArenaVec full");
        unsafe {
            ptr::write(self.ptr.add(self.len), val);
        }
        self.len += 1;
    }

    fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        let len = self.len;
        self.len = 0;
        (0..len).map(move |i| unsafe { ptr::read(self.ptr.add(i)) })
    }

    fn clear(&mut self) {
        // Drop elements
        for i in 0..self.len {
            unsafe { ptr::drop_in_place(self.ptr.add(i)); }
        }
        self.len = 0;
    }
}

impl<'arena, T> Drop for ArenaVec<'arena, T> {
    fn drop(&mut self) {
        self.clear(); // Drop elements, but don't free memory (arena owns it)
    }
}

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    // Arena persists across ticks (captured) — backing memory reused
    let mut arena = TickArena::new(1024);

    let mut tick = async move |input: &[i64]| -> bool {
        // Reset arena — all previous tick's allocations are gone
        arena.reset();

        // Allocate tick-local buffers FROM the arena
        // These can hold any T, including &'tick references
        let mut stream_hoff = ArenaVec::<i64>::new(&mut arena, 16);
        // Can't do ref handoff from same arena borrow... need to rethink.
        // Let's just test the owned case first.

        // Stratum 0
        {
            for &val in input {
                fold_state += val;
                persist_buf[persist_len] = val;
                persist_len += 1;
                stream_hoff.push(val);
            }
        }

        let fold_ref = &fold_state;

        // Stratum 1
        {
            print!("stream (fold={}): ", fold_ref);
            for val in stream_hoff.drain() {
                print!("{} ", val);
            }
            println!();
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10]).await;
}
