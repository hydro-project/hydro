//! Test 35: The cleanest unified approach — "Slab" per handoff.
//!
//! Summary of findings:
//! - For std: each handoff has a captured `Slab` (stores raw allocation).
//!   Each tick, reconstruct Vec<T> from the slab, use it, clear it, return it.
//!   T can be anything — owned or reference-containing. One codegen path.
//!
//! - For no_std: each handoff is a tick-local fixed array.
//!   No captured state needed (arrays are free to create on stack).
//!   T can be anything. One codegen path.
//!
//! - The codegen difference between std and no_std is just the buffer type:
//!   std:    `let mut hoff = slab.take::<T>();` ... `slab.reclaim(hoff);`
//!   no_std: `let mut hoff = [MaybeUninit::<T>::uninit(); N];`
//!
//! - Singleton refs are always just `let fold_ref = &fold_state;` — no buffer.
//!
//! This test shows the full picture with the Slab approach for std.
//! RESULT: ✅ PASS

/// Reusable allocation for a single handoff buffer.
/// Captured across ticks. Type-erased (no lifetime in storage).
/// 
/// For no_std, this type doesn't exist — replaced by tick-local arrays.
struct Slab {
    ptr: *mut u8,
    cap: usize,
    elem_layout: std::alloc::Layout,
}

impl Slab {
    fn new<T>() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            cap: 0,
            elem_layout: std::alloc::Layout::new::<T>(),
        }
    }

    /// Take the allocation as a Vec<T>. Vec starts empty, has previous capacity.
    /// SAFETY: T must have same Layout as the T used to create this Slab.
    unsafe fn take<T>(&mut self) -> Vec<T> {
        if self.ptr.is_null() {
            Vec::new()
        } else {
            let ptr = self.ptr as *mut T;
            let cap = self.cap;
            self.ptr = std::ptr::null_mut();
            self.cap = 0;
            unsafe { Vec::from_raw_parts(ptr, 0, cap) }
        }
    }

    /// Return the allocation. Vec must be empty (all elements consumed/dropped).
    fn reclaim<T>(&mut self, v: Vec<T>) {
        debug_assert!(v.is_empty());
        let (ptr, _, cap) = v.into_raw_parts();
        self.ptr = ptr as *mut u8;
        self.cap = cap;
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        if !self.ptr.is_null() && self.cap > 0 && self.elem_layout.size() > 0 {
            unsafe {
                let layout = std::alloc::Layout::from_size_align_unchecked(
                    self.elem_layout.size() * self.cap,
                    self.elem_layout.align(),
                );
                std::alloc::dealloc(self.ptr, layout);
            }
        }
    }
}

// ============================================================
// Simulated generated code (what the proc macro would produce)
// ============================================================

#[tokio::main]
async fn main() {
    // === Cross-tick state (captured) ===
    let mut fold_state: i64 = 0;                        // 'static fold
    let mut persist_buf: Vec<i64> = Vec::new();         // 'static persist
    let mut defer_buf: Vec<i64> = Vec::new();           // defer_tick (owned)
    let mut defer_back: Vec<i64> = Vec::new();          // defer_tick back-buffer

    // === Handoff slabs (captured, type-erased allocations) ===
    let mut slab_stream: Slab = Slab::new::<i64>();     // stream handoff
    let mut slab_refs: Slab = Slab::new::<&i64>();      // ref handoff (same Slab type!)

    // === Tick closure ===
    let mut tick = async move |input: &[i64]| -> bool {
        // --- Start of tick ---
        std::mem::swap(&mut defer_buf, &mut defer_back);

        // Reconstruct tick-local buffers from slabs (reuses allocation!)
        let mut stream_hoff: Vec<i64> = unsafe { slab_stream.take() };
        let mut ref_hoff: Vec<&i64> = unsafe { slab_refs.take() };

        // Stratum 0: source + fold + persist
        {
            // Drain deferred
            for val in defer_back.drain(..) {
                fold_state += val;
            }
            // Process input
            for &val in input {
                fold_state += val;
                persist_buf.push(val);
                stream_hoff.push(val);
            }
            // Fill ref handoff
            for item in persist_buf.iter() {
                ref_hoff.push(item);
            }
        }

        // Singleton ref (tick-local borrow, zero cost)
        let fold_ref: &i64 = &fold_state;

        // Stratum 1: consume handoffs + use singleton ref
        {
            print!("[tick] stream (fold={}): ", fold_ref);
            for val in stream_hoff.drain(..) {
                print!("{} ", val);
            }
            println!();

            print!("[tick] all persisted: ");
            for r in ref_hoff.iter() {
                print!("{} ", r);
            }
            println!();
        }

        // defer_tick output
        defer_buf.push(*fold_ref);

        // --- End of tick: return allocations to slabs ---
        ref_hoff.clear();
        slab_stream.reclaim(stream_hoff);
        slab_refs.reclaim(ref_hoff);

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10]).await;
    println!("---");
    tick(&[]).await; // empty input, defer_tick feeds
}
