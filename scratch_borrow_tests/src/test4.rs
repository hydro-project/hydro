//! Test 4: A handoff buffer that PERSISTS across ticks tries to hold &T.
//! This is the scenario from the last comment on #2713.
//! RESULT: ❌ FAIL — "closure implements FnMut, so references to captured variables can't escape"

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;
    let mut handoff: [Option<&i64>; 1] = [None]; // fixed-size "handoff"

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: fold, then put ref in handoff
        {
            fold_state += 1;
            handoff[0] = Some(&fold_state);
        }
        // Stratum 1: read from handoff
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
