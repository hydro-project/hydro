//! Test 11: Captured fixed-size buffer (ArrayVec-style) with owned values,
//! plus local reference to captured state. Best of both worlds for no_std?
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    // Captured fixed-size handoff - no alloc, persists across ticks
    let mut stream_hoff: [Option<i64>; 8] = [None; 8];
    let mut stream_len: usize = 0;

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: produce owned values into captured fixed buffer
        {
            fold_state += 1;
            stream_hoff[stream_len] = Some(fold_state);
            stream_len += 1;
            stream_hoff[stream_len] = Some(fold_state * 2);
            stream_len += 1;
        }

        // Local singleton ref to captured state
        let fold_ref: &i64 = &fold_state;

        // Stratum 1: drain captured buffer while borrowing fold_state
        {
            for i in 0..stream_len {
                if let Some(val) = stream_hoff[i].take() {
                    println!("val={}, fold_ref={}", val, fold_ref);
                }
            }
            stream_len = 0;
        }

        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
