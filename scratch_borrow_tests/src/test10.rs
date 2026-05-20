//! Test 10: Hybrid - captured Vec handoff (for perf, avoids realloc)
//! alongside local reference slots.
//! The Vec holds owned T, the local slot holds &T.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    // Captured handoff - persists for reuse, holds OWNED values
    let mut stream_hoff: Vec<i64> = Vec::new();

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: produce owned values into captured handoff
        {
            fold_state += 1;
            stream_hoff.push(fold_state);
            stream_hoff.push(fold_state * 2);
        }

        // Local singleton ref
        let fold_ref: &i64 = &fold_state;

        // Stratum 1: drain captured handoff while borrowing fold_state
        {
            for val in stream_hoff.drain(..) {
                println!("val={}, fold_ref={}", val, fold_ref);
            }
        }

        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
