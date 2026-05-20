//! Test 9: Realistic no_std scenario with external input via parameter.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    // Cross-tick state
    let mut fold_state: i64 = 0;
    let mut defer_buf: [Option<i64>; 4] = [None; 4];

    let mut tick = async move |input: &mut [Option<i64>; 4]| -> bool {
        // Tick-local handoff (fixed-size, no alloc)
        let mut stream_hoff: [Option<i64>; 4] = [None; 4];

        // Stratum 0: drain input, accumulate fold, forward to stream handoff
        {
            let mut idx = 0;
            for slot in input.iter_mut() {
                if let Some(val) = slot.take() {
                    fold_state += val;
                    stream_hoff[idx] = Some(val);
                    idx += 1;
                }
            }
        }

        // Singleton slot: borrow fold_state
        let singleton_ref: &i64 = &fold_state;

        // Stratum 1: cross_singleton equivalent - use ref alongside stream
        {
            for slot in stream_hoff.iter_mut() {
                if let Some(val) = slot.take() {
                    println!("item={}, fold_ref={}", val, singleton_ref);
                }
            }
        }

        // defer_tick: owned value into cross-tick buffer
        defer_buf[0] = Some(*singleton_ref);

        true
    };

    let mut input1 = [Some(1), Some(2), Some(3), None];
    tick(&mut input1).await;
    println!("---");
    let mut input2 = [Some(10), None, None, None];
    tick(&mut input2).await;
}
