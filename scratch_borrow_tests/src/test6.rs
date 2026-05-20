//! Test 6: Same as test5 but with async { }.await around subgraph blocks.
//! Does the async boundary break the borrowing?
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: i64 = 0;

    let mut tick = async move |_: &mut ()| -> bool {
        let mut handoff: [Option<&i64>; 1] = [None];

        // Stratum 0 (async)
        async {
            fold_state += 1;
            handoff[0] = Some(&fold_state);
        }.await;

        // Stratum 1 (async)
        async {
            if let Some(r) = handoff[0].take() {
                println!("got: {}", r);
            }
        }.await;

        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
