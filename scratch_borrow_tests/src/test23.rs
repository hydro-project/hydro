//! Test 23: raw_parts approach for reusing Vec<&T> allocation across ticks.
//! Sound because: vec is always empty between tick calls.
//! RESULT: ✅ PASS (requires unsafe)

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    // Store raw parts of a Vec<&i64> — erases the lifetime
    let (mut ptr, mut len, mut cap) = Vec::<&i64>::new().into_raw_parts();

    let mut tick = async move |input: &[i64]| -> bool {
        // Reconstruct Vec with tick-local lifetime
        let mut ref_handoff: Vec<&i64> = unsafe {
            debug_assert_eq!(len, 0, "ref handoff should be empty between ticks");
            Vec::from_raw_parts(ptr, len, cap)
        };

        // Stratum 0: persist + fill ref handoff
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
            for i in 0..persist_len {
                ref_handoff.push(&persist_buf[i]);
            }
        }

        // Stratum 1: consume refs
        {
            for r in ref_handoff.iter() {
                println!("ref: {}", r);
            }
        }

        // Clear and save raw parts back (refs are dead, vec is empty)
        ref_handoff.clear();
        let new_parts = ref_handoff.into_raw_parts();
        ptr = new_parts.0 as *mut &i64; // cast erases lifetime
        len = new_parts.1;
        cap = new_parts.2;

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
