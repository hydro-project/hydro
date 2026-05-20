//! Test 17: Full realistic no_std DFIR tick combining:
//! - Captured 'static fold state
//! - Captured defer_tick buffer (owned values, cross-tick)
//! - Tick-local stream handoff (owned values, within-tick)
//! - Tick-local singleton ref slot (&T to fold state)
//! - Tick-local ref handoff (&T to persist buffer)
//! All within async move ||, with async { }.await subgraph blocks.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    // === CAPTURED (cross-tick) state ===
    let mut fold_state: i64 = 0;
    let mut persist_buf: [i64; 16] = [0; 16];
    let mut persist_len: usize = 0;
    let mut defer_buf: [Option<i64>; 4] = [None; 4];
    let mut defer_back: [Option<i64>; 4] = [None; 4];

    let mut tick = async move |input: &[i64]| -> bool {
        // Double-buffer swap (start of tick)
        std::mem::swap(&mut defer_buf, &mut defer_back);

        // === TICK-LOCAL buffers ===
        let mut stream_hoff: [Option<i64>; 8] = [None; 8];
        let mut stream_len: usize = 0;

        // Stratum 0: source + fold + persist + forward to stream handoff
        async {
            // Drain deferred input from last tick
            for slot in defer_back.iter_mut() {
                if let Some(val) = slot.take() {
                    fold_state += val;
                }
            }
            // Process new input
            for &val in input {
                fold_state += val;
                persist_buf[persist_len] = val;
                persist_len += 1;
                stream_hoff[stream_len] = Some(val);
                stream_len += 1;
            }
        }.await;

        // Tick-local singleton ref
        let fold_ref: &i64 = &fold_state;

        // Tick-local ref handoff (references into persist_buf)
        let mut ref_hoff: [Option<&i64>; 16] = [None; 16];
        for i in 0..persist_len {
            ref_hoff[i] = Some(&persist_buf[i]);
        }

        // Stratum 1: use singleton ref + stream handoff + ref handoff
        async {
            print!("stream items (fold={}): ", fold_ref);
            for i in 0..stream_len {
                if let Some(val) = stream_hoff[i].take() {
                    print!("{} ", val);
                }
            }
            println!();

            print!("persisted refs: ");
            for slot in ref_hoff.iter() {
                if let Some(r) = slot {
                    print!("{} ", r);
                }
            }
            println!();
        }.await;

        // defer_tick: send fold_state to next tick
        defer_buf[0] = Some(*fold_ref);

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10]).await;
    println!("---");
    tick(&[]).await;
}
