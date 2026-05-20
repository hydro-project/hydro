//! Test 15: Fixed-size ref handoff with async { }.await between strata.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0 (async block)
        async {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
        }.await;

        // Tick-local ref handoff
        let mut ref_handoff: [Option<&i64>; 16] = [None; 16];
        async {
            for i in 0..persist_len {
                ref_handoff[i] = Some(&persist_buf[i]);
            }
        }.await;

        // Stratum 1 (async block)
        async {
            for slot in ref_handoff.iter() {
                if let Some(r) = slot {
                    println!("ref: {}", r);
                }
            }
        }.await;

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4, 5]).await;
}
