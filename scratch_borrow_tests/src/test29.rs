//! Test 29: Can we wrap the raw_parts pattern in a safe abstraction?
//! A "TickVec" that stores allocation across ticks but reconstructs each tick.
//! The key: the type parameter T is only present during the tick, not in storage.
//! RESULT: ✅ PASS

use std::marker::PhantomData;

/// Stores a Vec's allocation across tick boundaries without capturing T's lifetime.
/// Between ticks, this is just (ptr, cap) — no live T values exist.
struct HandoffAlloc {
    ptr: *mut u8,
    cap: usize,
    // Layout info for dealloc on drop
    align: usize,
    elem_size: usize,
}

impl HandoffAlloc {
    fn new<T>() -> Self {
        let v = Vec::<T>::new();
        let (ptr, _len, cap) = v.into_raw_parts();
        Self {
            ptr: ptr as *mut u8,
            cap,
            align: std::mem::align_of::<T>(),
            elem_size: std::mem::size_of::<T>(),
        }
    }

    /// Reconstruct a Vec<T> for this tick. Must be called with the same T
    /// that was used to create this HandoffAlloc (or same size/align).
    /// SAFETY: caller must ensure T matches the original layout and that
    /// the returned Vec is cleared before the tick ends.
    unsafe fn take_vec<T>(&mut self) -> Vec<T> {
        debug_assert_eq!(std::mem::size_of::<T>(), self.elem_size);
        debug_assert_eq!(std::mem::align_of::<T>(), self.align);
        Vec::from_raw_parts(self.ptr as *mut T, 0, self.cap)
    }

    /// Return the Vec's allocation after use. Vec must be empty.
    fn return_vec<T>(&mut self, v: Vec<T>) {
        assert!(v.is_empty(), "HandoffAlloc: vec must be empty when returned");
        let (ptr, _len, cap) = v.into_raw_parts();
        self.ptr = ptr as *mut u8;
        self.cap = cap;
    }
}

impl Drop for HandoffAlloc {
    fn drop(&mut self) {
        if self.cap > 0 && self.elem_size > 0 {
            unsafe {
                let layout = std::alloc::Layout::from_size_align_unchecked(
                    self.elem_size * self.cap,
                    self.align,
                );
                std::alloc::dealloc(self.ptr, layout);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    // One HandoffAlloc — doesn't know about lifetimes in T
    let mut alloc = HandoffAlloc::new::<&i64>();

    let mut tick = async move |input: &[i64]| -> bool {
        // Reconstruct Vec<&i64> for this tick
        let mut handoff: Vec<&i64> = unsafe { alloc.take_vec() };

        // Stratum 0
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
            for i in 0..persist_len {
                handoff.push(&persist_buf[i]);
            }
        }

        // Stratum 1
        {
            for r in handoff.iter() {
                println!("ref: {}", r);
            }
        }

        // Return allocation
        handoff.clear();
        alloc.return_vec(handoff);

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
