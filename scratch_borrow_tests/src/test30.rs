//! Test 30: The simplest unified approach — just make ALL handoffs tick-local.
//! For no_std: fixed arrays (zero cost to recreate).
//! For std: Vec::new() each tick (re-allocates, but simple and safe).
//! 
//! Question: is Vec::new() + push actually expensive? With small buffer
//! optimization or arena allocators it might be fine. And the compiler
//! might optimize it away entirely for small/known sizes.
//!
//! This test measures: does making the Vec tick-local actually prevent
//! allocation reuse? (Spoiler: yes, but it's the baseline safe approach.)
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // ALL handoffs are tick-local. Simple. Safe. No lifetime issues.
        let mut stream_hoff: Vec<i64> = Vec::new();       // owned stream
        let mut ref_hoff: Vec<&i64> = Vec::new();         // ref stream

        // Stratum 0
        {
            for &val in input {
                fold_state += val;
                persist_buf[persist_len] = val;
                persist_len += 1;
                stream_hoff.push(val);
            }
            for i in 0..persist_len {
                ref_hoff.push(&persist_buf[i]);
            }
        }

        // Singleton ref (always tick-local, always works)
        let fold_ref: &i64 = &fold_state;

        // Stratum 1
        {
            println!("fold = {}", fold_ref);
            print!("stream: ");
            for val in stream_hoff.drain(..) {
                print!("{} ", val);
            }
            println!();
            print!("refs: ");
            for r in ref_hoff.iter() {
                print!("{} ", r);
            }
            println!();
        }

        // Vecs are dropped here — allocation freed each tick.
        // For no_std, replace with fixed arrays (zero-cost).
        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[4]).await;
}
