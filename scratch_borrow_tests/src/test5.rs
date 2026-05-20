//! Test 5: Handoff buffer is LOCAL to each tick call (not captured across ticks).
//! For no_std: could we allocate fixed buffers on the stack each tick?
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;

    let mut tick = async move |_: &mut ()| -> bool {
        // Local to this tick call - not captured
        let mut handoff: [Option<&i64>; 1] = [None];

        // Stratum 0
        {
            fold_state += 1;
            handoff[0] = Some(&fold_state);
        }
        // Stratum 1
        {
            if let Some(r) = handoff[0].take() {
                println!("got: {}", r);
            }
        }
        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
