//! Test 3: Store &mut T in a local slot, use it in a later block.
//! Simulates mutable singleton reference.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut fold_state: Vec<i64> = Vec::new();

    let mut tick = async move |_: &mut ()| -> bool {
        // Stratum 0: fold accumulates
        fold_state.push(1);

        // Pass mutable ref to stratum 1
        let singleton_mut: &mut Vec<i64> = &mut fold_state;

        // Stratum 1: uses mutable ref
        {
            singleton_mut.push(99);
            println!("len = {}", singleton_mut.len());
        }
        true
    };

    tick(&mut ()).await;
    tick(&mut ()).await;
}
