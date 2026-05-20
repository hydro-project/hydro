//! Test 16: Can we have a CAPTURED fixed buffer that we refill with &T each tick?
//! RESULT: ❌ FAIL — same FnMut wall as test 4

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;
    // Captured handoff - but wants to hold &i64 pointing into persist_buf
    let mut ref_handoff: [Option<&i64>; 16] = [None; 16];

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
        }

        // Fill captured ref handoff with refs to captured persist_buf
        {
            for i in 0..persist_len {
                ref_handoff[i] = Some(&persist_buf[i]);
            }
        }

        // Stratum 1
        {
            for slot in ref_handoff.iter_mut() {
                if let Some(r) = slot.take() {
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
