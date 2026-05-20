//! Test 27: Unified codegen path — raw_parts as a GENERAL mechanism for all handoffs.
//! The idea: every handoff stores its allocation as raw parts (ptr, len, cap).
//! Each tick, reconstruct the Vec<T> locally. T can be anything — owned, &'a, etc.
//! The codegen doesn't need to know whether T contains references.
//! RESULT: ✅ PASS
//!
//! This is the "one codegen path" approach: same code whether T = i64 or T = &'a i64.

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;

    // Handoff stored as raw parts — type-erased at the lifetime level.
    // For owned types (i64), this is just a Vec that persists.
    // For ref types (&i64), this reuses the allocation without capturing the lifetime.
    let mut hoff_parts: (*mut u8, usize, usize) = {
        let v = Vec::<i64>::new();
        let (ptr, len, cap) = v.into_raw_parts();
        (ptr as *mut u8, len, cap)
    };

    let mut tick = async move |input: &[i64]| -> bool {
        // Reconstruct the Vec locally each tick.
        // SAFETY: parts came from a Vec<i64>, len is always 0 between ticks.
        let mut handoff: Vec<i64> = unsafe {
            Vec::from_raw_parts(hoff_parts.0 as *mut i64, hoff_parts.1, hoff_parts.2)
        };

        // Stratum 0: produce
        {
            for &val in input {
                fold_state += val;
                handoff.push(val);
            }
        }

        // Stratum 1: consume
        {
            let fold_ref = &fold_state;
            for val in handoff.drain(..) {
                println!("item={}, fold={}", val, fold_ref);
            }
        }

        // Save parts back (vec is empty, allocation preserved)
        let (ptr, len, cap) = handoff.into_raw_parts();
        hoff_parts = (ptr as *mut u8, len, cap);

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10]).await;
}
