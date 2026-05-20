//! Test 14: Fixed-size handoff carrying &T references. No alloc.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0: persist
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
        }

        // Tick-local fixed handoff carrying references
        let mut ref_handoff: [Option<&i64>; 16] = [None; 16];
        {
            for i in 0..persist_len {
                ref_handoff[i] = Some(&persist_buf[i]);
            }
        }

        // Stratum 1: consume references
        {
            for slot in ref_handoff.iter() {
                if let Some(r) = slot {
                    println!("ref: {}", r);
                }
            }
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4, 5]).await;
}
