//! Test 28: Same unified raw_parts approach, but T = &'tick i64 (reference type).
//! Demonstrates that the SAME codegen pattern works for reference-containing types.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    // Same raw_parts pattern — but this time the Vec will hold &i64.
    // The codegen is identical; it doesn't need to know T contains a lifetime.
    let mut hoff_parts: (*mut u8, usize, usize) = {
        let v = Vec::<&i64>::new();
        let (ptr, len, cap) = v.into_raw_parts();
        (ptr as *mut u8, len, cap)
    };

    let mut tick = async move |input: &[i64]| -> bool {
        // Reconstruct Vec<&i64> locally — lifetime is tick-scoped
        let mut handoff: Vec<&i64> = unsafe {
            Vec::from_raw_parts(hoff_parts.0 as *mut &i64, hoff_parts.1, hoff_parts.2)
        };

        // Stratum 0: persist + fill ref handoff
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
            for i in 0..persist_len {
                handoff.push(&persist_buf[i]);
            }
        }

        // Stratum 1: consume references
        {
            for r in handoff.iter() {
                println!("ref: {}", r);
            }
        }

        // Clear and save parts (refs are dead, vec is empty)
        handoff.clear();
        let (ptr, len, cap) = handoff.into_raw_parts();
        hoff_parts = (ptr as *mut u8, len, cap);

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
