//! Test 24: Captured Vec<&'static T> with lifetime transmute.
//! Simpler than raw_parts but equally unsafe.
//! RESULT: ✅ PASS (requires unsafe)

#[tokio::main]
async fn main() {
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;
    // Captured with 'static lifetime — we'll transmute real refs into this
    let mut ref_handoff: Vec<&'static i64> = Vec::new();

    let mut tick = async move |input: &[i64]| -> bool {
        ref_handoff.clear();

        // Stratum 0
        {
            for &val in input {
                persist_buf[persist_len] = val;
                persist_len += 1;
            }
            for i in 0..persist_len {
                let r: &i64 = &persist_buf[i];
                // SAFETY: ref is valid for the duration of this tick call,
                // and we clear the vec before returning.
                let r_static: &'static i64 = unsafe { std::mem::transmute(r) };
                ref_handoff.push(r_static);
            }
        }

        // Stratum 1
        {
            for r in ref_handoff.iter() {
                println!("ref: {}", r);
            }
        }

        ref_handoff.clear(); // MUST clear before tick returns
        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
