//! Test 22: Index-based ref handoff. Captured Vec<usize> holds indices,
//! resolved to &T at consumption time. Reuses allocation, no unsafe.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;
    // Captured "ref handoff" — but stores indices, not references
    let mut ref_handoff_indices: Vec<usize> = Vec::new();

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0: persist
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
            // "Emit references" = emit indices
            ref_handoff_indices.clear();
            for i in 0..persist_len {
                ref_handoff_indices.push(i);
            }
        }

        // Stratum 1: resolve indices to &T
        {
            for &idx in ref_handoff_indices.iter() {
                let r: &i64 = &persist_buf[idx];
                println!("ref via index: {}", r);
            }
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
