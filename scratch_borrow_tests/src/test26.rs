//! Test 26: Captured fixed array used as stream handoff (owned values).
//! Cleared at start of tick. No lifetime issues since it holds owned T.
//! This is the no_std stream handoff pattern.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    // Captured fixed-size stream handoff (owned values, no refs)
    let mut stream_hoff: [i64; 8] = [0; 8];
    let mut stream_hoff_len: usize = 0;

    let mut tick = async move |input: &[i64]| -> bool {
        // Clear from last tick
        stream_hoff_len = 0;

        // Stratum 0: produce owned values
        {
            for &val in input {
                fold_state += val;
                stream_hoff[stream_hoff_len] = val;
                stream_hoff_len += 1;
            }
        }

        // Singleton ref (tick-local borrow of captured state)
        let fold_ref: &i64 = &fold_state;

        // Stratum 1: consume owned values from handoff + borrow singleton
        {
            for i in 0..stream_hoff_len {
                println!("item={}, fold={}", stream_hoff[i], fold_ref);
            }
        }

        true
    };

    tick(&[1, 2, 3]).await;
    println!("---");
    tick(&[10, 20]).await;
}
