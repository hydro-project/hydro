//! Test 12: Can fold's pipe output be &mut Acc flowing into a downstream operator?
//! This is the 'static fold case where we want zero-copy output.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: Vec<i64> = Vec::new();

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: fold accumulates (mutates captured state)
        {
            fold_state.push(1);
        }

        // The fold's "output" is &mut Vec<i64> - can downstream use it?
        let fold_output: &mut Vec<i64> = &mut fold_state;

        // Stratum 1: downstream operator receives &mut Acc
        {
            // e.g., map over the accumulated vec
            let sum: i64 = fold_output.iter().sum();
            println!("sum of fold state = {}", sum);
            // Can even mutate it (for state() operator semantics)
            fold_output.retain(|x| *x > 0);
        }

        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
