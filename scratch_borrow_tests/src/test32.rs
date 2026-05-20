//! Test 32: Simpler approach — a "SlabVec" that's just a pre-allocated Vec
//! that gets cleared and reused, but the Vec itself is reconstructed tick-locally
//! from a captured raw allocation. No arena needed — each handoff owns its slab.
//!
//! Key insight: we don't need a shared arena. Each handoff can independently
//! store its own allocation as raw parts. The "allocator" is just the pattern of:
//! 1. Captured: (ptr, cap) — the raw allocation, no type/lifetime info
//! 2. Tick-local: reconstruct Vec<T> from (ptr, 0, cap), use it, clear it, save back
//!
//! This is test 27/28 but wrapped in a clean abstraction.
//! RESULT: ✅ PASS

/// Stores a Vec's heap allocation across tick boundaries.
/// T is erased — only size/align/ptr/cap are stored.
/// Between ticks, no T values exist (len is always 0).
struct Slab {
    ptr: *mut u8,
    cap: usize,
    elem_size: usize,
    elem_align: usize,
}

impl Slab {
    /// Create a new empty slab for elements of type T.
    fn new<T>() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            cap: 0,
            elem_size: std::mem::size_of::<T>(),
            elem_align: std::mem::align_of::<T>(),
        }
    }

    /// Reconstruct a Vec<T> for this tick. The Vec starts empty but may have
    /// pre-allocated capacity from previous ticks.
    /// SAFETY: T must match the original type's size and alignment.
    /// The returned Vec must be cleared before being passed back to `reclaim`.
    unsafe fn take<T>(&mut self) -> Vec<T> {
        debug_assert_eq!(std::mem::size_of::<T>(), self.elem_size);
        debug_assert_eq!(std::mem::align_of::<T>(), self.elem_align);
        if self.ptr.is_null() {
            Vec::new()
        } else {
            let v = Vec::from_raw_parts(self.ptr as *mut T, 0, self.cap);
            self.ptr = std::ptr::null_mut();
            self.cap = 0;
            v
        }
    }

    /// Return a Vec's allocation to the slab. Vec must be empty.
    fn reclaim<T>(&mut self, v: Vec<T>) {
        debug_assert!(v.is_empty());
        let (ptr, _len, cap) = v.into_raw_parts();
        self.ptr = ptr as *mut u8;
        self.cap = cap;
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.cap > 0 && self.elem_size > 0 {
            unsafe {
                let layout = std::alloc::Layout::from_size_align_unchecked(
                    self.elem_size * self.cap,
                    self.elem_align,
                );
                std::alloc::dealloc(self.ptr, layout);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    // Captured slabs — one per handoff. Type-erased, no lifetime in storage.
    let mut stream_slab = Slab::new::<i64>();
    let mut ref_slab = Slab::new::<&i64>(); // same Slab type works for refs!

    let mut tick = async move |input: &[i64]| -> bool {
        // Reconstruct tick-local Vecs from slabs
        let mut stream_hoff: Vec<i64> = unsafe { stream_slab.take() };
        let mut ref_hoff: Vec<&i64> = unsafe { ref_slab.take() };

        // Stratum 0
        {
            for &val in input {
                fold_state += val;
                persist_buf[persist_len] = val;
                persist_len += 1;
                stream_hoff.push(val);
            }
            for i in 0..persist_len {
                ref_hoff.push(&persist_buf[i]);
            }
        }

        let fold_ref = &fold_state;

        // Stratum 1
        {
            print!("stream (fold={}): ", fold_ref);
            for val in stream_hoff.drain(..) {
                print!("{} ", val);
            }
            println!();
            print!("refs: ");
            for r in ref_hoff.iter() {
                print!("{} ", r);
            }
            println!();
        }

        // Return allocations to slabs
        ref_hoff.clear();
        stream_slab.reclaim(stream_hoff);
        ref_slab.reclaim(ref_hoff);

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
